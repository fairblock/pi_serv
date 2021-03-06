use pi_vm::bonmgr::{BonMgr, StructMeta, FnMeta, jstype_ptr,ptr_jstype, CallResult};
use pi_vm::adapter::{JSType, JS};
use std::sync::Arc;
use std::mem::transmute;
use atom::Atom;
use guid;



fn call_596751388(js: Arc<JS>, v:Vec<JSType>) -> Option<CallResult>{
	let param_error = "param error in new";

	let jst0 = &v[0];
    if !jst0.is_uint8_array() && !jst0.is_array_buffer(){return Some(CallResult::Err(String::from(param_error))); }
    let arr = unsafe{*(jst0.to_bytes().as_ptr() as usize as *const [u8; 8])};
    let jst0 = unsafe {
        transmute::<[u8; 8], u64>(arr)
    }; 


	let jst1 = &v[1];
	if !jst1.is_number(){ return Some(CallResult::Err(String::from(param_error)));}
	let jst1 = jst1.get_u16();


    let result = guid::GuidGen::new(jst0,jst1);
    let ptr = Box::into_raw(Box::new(result)) as usize;let mut result = ptr_jstype(js.get_objs(), js.clone(), ptr,1736136244);


    Some(CallResult::Ok)
}

fn drop_1736136244(ptr: usize){
    unsafe { Box::from_raw(ptr as *mut guid::GuidGen) };
}
pub fn register(mgr: &BonMgr){
    mgr.regist_struct_meta(StructMeta{name:String::from("guid::GuidGen"), drop_fn: drop_1736136244}, 1736136244);
    mgr.regist_fun_meta(FnMeta::CallArg(call_596751388), 596751388);
}