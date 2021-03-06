use pi_vm::bonmgr::{BonMgr, StructMeta, FnMeta, jstype_ptr,ptr_jstype, CallResult};
use pi_vm::adapter::{JSType, JS};
use std::sync::Arc;
use pi_vm::pi_vm_impl::{ block_reply, block_throw, push_callback};
use worker::task::TaskType;
use atom::Atom;
use std::mem::{transmute, uninitialized};
use pi_store;



fn call_4027749383(js: Arc<JS>, v:Vec<JSType>) -> Option<CallResult>{
	let param_error = "param error in new";

	let jst0 = &v[0];
    if !jst0.is_string(){ return Some(CallResult::Err(String::from(param_error)));}
    let jst0 = Atom::from(jst0.get_str());


	let jst1 = &v[1];
	if !jst1.is_number(){ return Some(CallResult::Err(String::from(param_error)));}
	let jst1 = jst1.get_u32() as usize;


    let result = pi_store::lmdb_file::DB::new(jst0,jst1);let mut result = match result{
        Ok(r) => { 
    let ptr = Box::into_raw(Box::new(r)) as usize;let mut r = ptr_jstype(js.get_objs(), js.clone(), ptr,568147534);

 r }
        Err(v) => { 
            return Some(CallResult::Err(v + ", Result is Err"));
        }
    };

    Some(CallResult::Ok)
}


fn call_2763865666(js: Arc<JS>, v:Vec<JSType>) -> Option<CallResult>{
	let param_error = "param error in new";

	let jst0 = &v[0];
    if !jst0.is_string(){ return Some(CallResult::Err(String::from(param_error)));}
    let jst0 = Atom::from(jst0.get_str());


	let jst1 = &v[1];
	if !jst1.is_number(){ return Some(CallResult::Err(String::from(param_error)));}
	let jst1 = jst1.get_u32() as usize;


    let result = pi_store::file_mem_db::FileMemDB::new(jst0,jst1);
    let ptr = Box::into_raw(Box::new(result)) as usize;let mut result = ptr_jstype(js.get_objs(), js.clone(), ptr,2325173571);


    Some(CallResult::Ok)
}


fn call_2467240184(js: Arc<JS>, v:Vec<JSType>) -> Option<CallResult>{
	let param_error = "param error in new";

	let jst0 = &v[0];
    if !jst0.is_string(){ return Some(CallResult::Err(String::from(param_error)));}
    let jst0 = Atom::from(jst0.get_str());


	let jst1 = &v[1];
	if !jst1.is_number(){ return Some(CallResult::Err(String::from(param_error)));}
	let jst1 = jst1.get_u32() as usize;


    let result = pi_store::log_file_db::LogFileDB::new(jst0,jst1);
    let ptr = Box::into_raw(Box::new(result)) as usize;let mut result = ptr_jstype(js.get_objs(), js.clone(), ptr,1492732803);


    Some(CallResult::Ok)
}

fn drop_568147534(ptr: usize){
    unsafe { Box::from_raw(ptr as *mut pi_store::lmdb_file::DB) };
}

fn drop_2325173571(ptr: usize){
    unsafe { Box::from_raw(ptr as *mut pi_store::file_mem_db::FileMemDB) };
}

fn drop_1492732803(ptr: usize){
    unsafe { Box::from_raw(ptr as *mut pi_store::log_file_db::LogFileDB) };
}
pub fn register(mgr: &BonMgr){
    mgr.regist_struct_meta(StructMeta{name:String::from("pi_store::lmdb_file::DB"), drop_fn: drop_568147534}, 568147534);
    mgr.regist_struct_meta(StructMeta{name:String::from("pi_store::file_mem_db::FileMemDB"), drop_fn: drop_2325173571}, 2325173571);
    mgr.regist_struct_meta(StructMeta{name:String::from("pi_store::log_file_db::LogFileDB"), drop_fn: drop_1492732803}, 1492732803);
    mgr.regist_fun_meta(FnMeta::CallArg(call_4027749383), 4027749383);
    mgr.regist_fun_meta(FnMeta::CallArg(call_2763865666), 2763865666);
    mgr.regist_fun_meta(FnMeta::CallArg(call_2467240184), 2467240184);
}