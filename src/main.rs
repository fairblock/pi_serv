#![allow(unused_imports)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use clap::{App, Arg, ArgMatches, SubCommand};
use env_logger;
use num_cpus::get_physical;
use parking_lot::{Condvar, Mutex, RwLock, WaitTimeoutResult};

use hash::XHashMap;
use r#async::rt::{
    multi_thread::{MultiTaskPool, MultiTaskRuntime},
    single_thread::{SingleTaskRunner, SingleTaskRuntime},
    AsyncRuntime,
};
use tcp::{
    buffer_pool::WriteBufferPool,
    connect::TcpSocket,
    driver::SocketConfig,
    server::{AsyncPortsFactory, SocketListener},
};
use vm_builtin::{ContextHandle, VmStartupSnapshot};
use vm_core::{debug, init_v8, vm, worker};
use ws::server::WebsocketListenerFactory;

lazy_static! {
    //主线程运行状态和线程无条件休眠超时时长
    static ref MAIN_RUN_STATUS: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    static ref MAIN_UNCONDITION_SLEEP_TIMEOUT: u64 = 1;

    //主线程条件变量和线程条件休眠超时时长
    static ref MAIN_CONDVAR: Arc<(AtomicBool, Mutex<()>, Condvar)> = Arc::new((AtomicBool::new(false), Mutex::new(()), Condvar::new()));
    static ref MAIN_CONDITION_SLEEP_TIMEOUT: u64 = 10000;

    //初始化主线程异步运行时
    static ref MAIN_ASYNC_RUNNER: SingleTaskRunner<()> = SingleTaskRunner::new();
    static ref MAIN_ASYNC_RUNTIME: SingleTaskRuntime<()> = MAIN_ASYNC_RUNNER.startup().unwrap();

    //初始化文件异步运行时
    static ref FILES_ASYNC_RUNTIME: MultiTaskRuntime<()> = {
        let pool = MultiTaskPool::new("PI-SERV-FILE".to_string(), get_physical(), 2 * 1024 * 1024, 10, Some(10));
        pool.startup(false)
    };
}

/*
* 同步执行入口，退出时会中止主线程
*/
fn main() {
    //初始化日志服务器
    env_logger::init();

    //匹配启动时的选项和参数
    let matches = App::new("Pi Serv Main")
        .version("0.2.0")
        .author("YiNeng <yineng@foxmail.com>")
        .arg(
            Arg::with_name("INIT_HEAP_SIZE") //虚拟机初始堆大小
                .short("I")
                .long("INIT_HEAP_SIZE")
                .value_name("Mbytes")
                .help("Set init vm heap size")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("MAX_HEAP_SIZE") //虚拟机最大堆大小
                .short("H")
                .long("MAX_HEAP_SIZE")
                .value_name("Mbytes")
                .help("Set max vm heap size")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("WORK_VM_MULTIPLE") //工作虚拟机倍数
                .short("W")
                .long("WORK_VM_MULTIPLE")
                .value_name("Multiple")
                .help("Set multiple of work vm amount")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("DEBUG") //工作虚拟机调试模式
                .short("D")
                .long("DEBUG")
                .value_name("Port")
                .help("Enable debug work vm on port")
                .takes_value(true),
        )
        .get_matches();

    //初始化V8环境，并启动初始虚拟机
    let (init_heap_size, max_heap_size, debug_port) = init_v8_env(&matches);
    let init_vm = create_init_vm(init_heap_size, max_heap_size, debug_port);

    //主线程循环
    let matches_copy = matches.clone();
    let init_vm_copy = init_vm.clone();
    if let Err(e) = MAIN_ASYNC_RUNTIME.spawn(MAIN_ASYNC_RUNTIME.alloc(), async move {
        async_main(
            matches_copy,
            init_vm_copy,
            init_heap_size,
            max_heap_size,
            debug_port,
        )
        .await;
    }) {
        panic!("Spawn async main task failed, reason: {:?}", e);
    }
    while MAIN_RUN_STATUS.load(Ordering::Relaxed) {
        //推动主线程异步运行时
        if let Err(e) = MAIN_ASYNC_RUNNER.run() {
            panic!("Main loop failed, reason: {:?}", e);
        }

        //推动初始虚拟机
        let run_time = init_vm.run();

        if MAIN_ASYNC_RUNTIME.len() == 0 && init_vm.queue_len() == 0 {
            //当前没有主线程任务，则休眠主线程
            let (is_sleep, lock, condvar) = &**MAIN_CONDVAR;
            let mut locked = lock.lock();
            if !is_sleep.load(Ordering::Relaxed) {
                //如果当前未休眠，则休眠
                is_sleep.store(true, Ordering::SeqCst);
                if condvar
                    .wait_for(
                        &mut locked,
                        Duration::from_millis(*MAIN_CONDITION_SLEEP_TIMEOUT),
                    )
                    .timed_out()
                {
                    //条件超时唤醒，则设置状态为未休眠
                    is_sleep.store(false, Ordering::SeqCst);
                }
            }
        } else {
            //当前主线程有任务
            if let Some(remaining_interval) =
                Duration::from_millis(*MAIN_UNCONDITION_SLEEP_TIMEOUT).checked_sub(run_time)
            {
                //本次运行少于循环间隔，则休眠剩余的循环间隔，并继续执行任务
                thread::sleep(remaining_interval);
            }
        }
    }
}

//初始化V8环境，如果是调试模式则返回调试端口
fn init_v8_env(matches: &ArgMatches) -> (usize, usize, Option<u16>) {
    let mut init_heap_size = 16 * 1024 * 1024; //默认虚拟机初始堆大小为16MB
    if let Some(size) = matches.value_of("INIT_HEAP_SIZE") {
        match size.parse::<usize>() {
            Err(e) => panic!("Init v8 env failed, reason: {:?}", e),
            Ok(num) => {
                if num.is_power_of_two() {
                    init_heap_size = num * 1024 * 1024;
                }
            }
        }
    }

    let mut max_heap_size = 8096 * 1024 * 1024; //默认虚拟机最大堆大小为8GB
    if let Some(size) = matches.value_of("MAX_HEAP_SIZE") {
        match size.parse::<usize>() {
            Err(e) => panic!("Init v8 env failed, reason: {:?}", e),
            Ok(num) => {
                if num.is_power_of_two() {
                    max_heap_size = num * 1024 * 1024;
                }
            }
        }
    }

    let mut debug_port: u16 = 0;
    if let Some(value) = matches.value_of("DEBUG") {
        match value.parse::<u16>() {
            Err(e) => {
                panic!("Bind debug listene port failed, reason: {:?}", e);
            }
            Ok(port) => {
                if port > 1024 {
                    debug_port = port;
                } else {
                    panic!(
                        "Bind debug listene port failed, port: {}, reason: invalid port",
                        port
                    );
                }
            }
        }
    }

    init_v8(Some(vec![
        "".to_string(),
        "--no-wasm-async-compilation".to_string(),
        "--harmony-top-level-await".to_string(),
        "--expose-gc".to_string(),
    ]));

    if debug_port > 0 {
        //启动虚拟机调试用Websocket服务
        let ws_server_factory = Arc::new(debug::InspectorWebsocketServerFactory::new::<PathBuf>(
            "", None,
        )); // TODO: 浏览器的调试不能指定路径，但是vscode的调试需要指定路径
        let mut factory = AsyncPortsFactory::<TcpSocket>::new();
        factory.bind(
            debug_port,
            Box::new(
                WebsocketListenerFactory::<TcpSocket>::with_protocol_factory(ws_server_factory),
            ),
        );
        let mut config = SocketConfig::new("0.0.0.0", factory.bind_ports().as_slice());
        config.set_option(16384, 16384, 16384, 16);
        let buffer = WriteBufferPool::new(1000, 10, 3).ok().unwrap();
        match SocketListener::bind_with_processor(
            factory,
            buffer,
            config,
            1,
            1024,
            2 * 1024 * 1024,
            1024,
            Some(10),
        ) {
            Err(e) => {
                panic!(
                    "Bind debug listene port failed, port: {}, reason: {:?}",
                    debug_port, e
                );
            }
            Ok(_) => {
                info!("Bind debug listene port ok, port: {}", debug_port);
            }
        }
    }

    info!("Init v8 env ok");
    (init_heap_size, max_heap_size, Some(debug_port))
}

//创建初始虚拟机
fn create_init_vm(init_heap_size: usize, max_heap_size: usize, debug_port: Option<u16>) -> vm::Vm {
    let mut builder = vm::VmBuilder::new().snapshot_template();
    builder = builder.heap_limit(init_heap_size, max_heap_size);

    if debug_port.is_some() {
        //允许调试
        builder = builder.enable_inspect();
    }

    builder.bind_condvar_waker(MAIN_CONDVAR.clone()).build()
}

/*
* 异步执行入口，退出时不会中止主线程
*/
async fn async_main(
    matches: ArgMatches<'static>,
    init_vm: vm::Vm,
    init_heap_size: usize,
    max_heap_size: usize,
    debug_port: Option<u16>,
) {
    let snapshot_context = init_snapshot(&init_vm).await;

    //TODO 加载项目的入口模块文件, 并加载其静态依赖树中的所有js模块文件

    finish_snapshot(&init_vm, snapshot_context).await;

    let workers = init_work_vm(
        &matches,
        &init_vm,
        init_heap_size,
        max_heap_size,
        debug_port,
    );
    workers[0]
        .1
        .new_context(None, workers[0].1.alloc_context_id(), None)
        .await
        .unwrap();
}

//初始化快照
async fn init_snapshot(init_vm: &vm::Vm) -> ContextHandle {
    let snapshot_context = init_vm
        .new_context(None, init_vm.alloc_context_id(), None)
        .await
        .unwrap();
    if let Err(e) = init_vm
        .execute(
            snapshot_context,
            "Init_Vm_Init_module.js",
            r#"
                    onerror = function(e) {
                        console.log("catch global error, e:", e.stack);
                    };
                "#,
        )
        .await
    {
        panic!("!!!!!!Init snapshot failed, reason: {:?}", e);
    }
    info!("Init snapshot ok");

    snapshot_context
}

//完成快照
async fn finish_snapshot(init_vm: &vm::Vm, snapshot_context: ContextHandle) {
    if let Err(e) = init_vm.snapshot(snapshot_context).await {
        panic!("!!!!!!Finish snapshot failed, reason: {:?}", e);
    }
    info!("Snapshot finish");
}

//初始化工作虚拟机，返回工作虚拟机
fn init_work_vm(
    matches: &ArgMatches,
    init_vm: &vm::Vm,
    init_heap_size: usize,
    max_heap_size: usize,
    debug_port: Option<u16>,
) -> Vec<(Arc<AtomicBool>, vm::Vm)> {
    let mut work_vm_count: usize = 2 * get_physical(); //默认工作虚拟机数量为本地cpu物理核数的2倍
    if let Some(value) = matches.value_of("WORK_VM_MULTIPLE") {
        match value.parse::<usize>() {
            Err(e) => {
                panic!("Init work vm failed, reason: {:?}", e);
            }
            Ok(count) => {
                work_vm_count = get_physical() * count;
            }
        }
    }

    let mut vec = Vec::with_capacity(work_vm_count);
    let snapshot = VmStartupSnapshot::Snapshot(init_vm.get_snapshot().unwrap());
    let snapshot_bytes = snapshot.bytes();
    for index in 0..work_vm_count {
        //使用指定快照，创建工作虚拟机
        let work_vm_snapshot = VmStartupSnapshot::Boxed(snapshot_bytes.to_vec().into_boxed_slice());
        let mut builder = vm::VmBuilder::new().startup_snapshot(work_vm_snapshot);
        builder = builder.heap_limit(init_heap_size, max_heap_size);

        if debug_port.is_some() {
            //允许调试
            builder = builder.enable_inspect();
        }
        let work_vm = builder.build();
        let work_vm_copy = work_vm.clone();

        //启动工作线程，并运行工作虚拟机
        let worker_name = "PI-SERV-WORKER".to_string() + index.to_string().as_str();
        info!(
            "Worker ready, thread: {}, worker: {}",
            worker_name,
            "Vm-".to_string() + work_vm.get_vid().to_string().as_str()
        );

        let worker_handle = worker::spawn_worker_thread(
            worker_name.as_str(),
            2 * 1024 * 1024,
            Arc::new((AtomicBool::new(false), Mutex::new(()), Condvar::new())),
            1,
            None,
            move || {
                let run_time = work_vm.run();
                (work_vm.queue_len() == 0, run_time)
            },
        );

        vec.push((worker_handle, work_vm_copy));
    }

    vec
}
