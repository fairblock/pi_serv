use pi_vm::bonmgr::{BonMgr, FnMeta, jstype_ptr, CallResult};
use pi_vm::adapter::{JSType, JS};
use pi_vm::channel_map::VMChannel;
use pi_vm::pi_vm_impl::async_request;
use std::sync::Arc;

fn async_request_hash(js: Arc<JS>, v:Vec<JSType>) -> Option<CallResult>{
	let param_error = "param error in async_request";

	let jst0 = &v[0];
	if !jst0.is_string(){return Some(CallResult::Err(String::from(param_error))); }
    let jst0 = jst0.get_str();

    let jst1 = &v[1];
	if !jst1.is_uint8_array() && !jst1.is_array_buffer(){return Some(CallResult::Err(String::from(param_error))); }
    let jst1 = jst1.to_bytes();

	let jst2 = &v[2];
	if !jst2.is_array(){return Some(CallResult::Err(String::from(param_error)));}
    let len = jst2.get_array_length();
    let mut arr = Vec::with_capacity(len);
    for i in 0..len{
        arr.push(jst2.get_index(i as u32));
    }
    
    let jst3 = &v[3];
    if !jst3.is_number(){return Some(CallResult::Err(String::from(param_error)));}
    let jst3 = jst3.get_u32();

    //let result = async_request(js.clone(), jst0, jst1, jst2, jst3);
    js.new_boolean(true);

    Some(CallResult::Ok)
}

fn async_response_hash(js: Arc<JS>, v:Vec<JSType>) -> Option<CallResult>{
	let param_error = "param error in async_response_hash";

    //VMChannel
    let jst0 = &v[0];
    let ptr = jstype_ptr(&jst0, js.clone(), 3366364668, true, param_error).expect("");
	let jst0 = *unsafe { Box::from_raw(ptr as *mut VMChannel) };

    //args
    let jst1 = &v[1];
	if !jst1.is_uint8_array() && !jst1.is_array_buffer(){return Some(CallResult::Err(String::from(param_error))); }
    let jst1 = jst1.to_bytes();

    //&[nativObject]
	let jst2 = &v[2];
	if !jst2.is_array(){return Some(CallResult::Err(String::from(param_error)));}
    let len = jst2.get_array_length();
    let mut arr = Vec::with_capacity(len);
    for i in 0..len{
        arr.push(jst2.get_index(i as u32));
    }
    
    //callbackIndex
    let jst3 = &v[3];
    if !jst3.is_number(){return Some(CallResult::Err(String::from(param_error)));}
    let jst3 = jst3.get_u32();

    //let result = jst0.response(jst3, jst1, jst2);
    js.new_boolean(true);

    Some(CallResult::Ok)
}

pub fn register(mgr: &BonMgr){
    mgr.regist_fun_meta(FnMeta::CallArg(async_request_hash), 1);
    mgr.regist_fun_meta(FnMeta::CallArg(async_response_hash), 2);
}