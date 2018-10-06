extern crate serde_json;
use context::Context;
use holochain_dna::wasm::DnaWasm;
use nucleus::{
    ribosome::{
        self,
        callback::{CallbackResult, get_dna}
    },
    ZomeFnCall,
};
use std::sync::Arc;
use holochain_wasm_utils::{
    validation::{
        ValidationData, HcEntryLifecycle, HcEntryAction
    }
};
use hash_table::{entry::Entry, sys_entry::EntryType};

pub fn validate_entry(
    entry: Entry,
    entry_type: EntryType,
    validation_data: ValidationData,
    context: Arc<Context>,
) -> CallbackResult {
    println!("VALIDATE_ENTRY match: {}", entry_type.as_str());
    match entry_type {
        EntryType::App(app_entry_type) => validate_app_entry(entry, app_entry_type, validation_data, context),
        EntryType::Dna => CallbackResult::Pass,
        _ => CallbackResult::NotImplemented
    }
}

fn validate_app_entry(
    entry: Entry,
    app_entry_type: String,
    validation_data: ValidationData,
    context: Arc<Context>,
) -> CallbackResult {
    println!("VALIDATE_APP_ENTRY");
    let dna = get_dna(&context).expect("Callback called without DNA set!");
    let zome_name = dna.get_zome_name_for_entry_type(&app_entry_type);
    if zome_name.is_none() {
        println!("VALIDATE_APP_ENTRY: no zome for entry type {}", app_entry_type);
        return CallbackResult::NotImplemented
    }

    let zome_name = zome_name.unwrap();
    match get_wasm(&context, &zome_name) {
        Some(wasm) => {
            println!("VALIDATE_APP_ENTRY: wasm found!");
            let validation_call = build_validation_call(entry, app_entry_type, zome_name, validation_data);
            run_validation_callback(context.clone(), validation_call, &wasm, dna.name.clone())
        },
        None => {
            println!("VALIDATE_APP_ENTRY: no wasm found for zome {}!", zome_name);
            CallbackResult::NotImplemented
        },
    }
}

fn build_validation_call(entry : Entry, entry_type: String, zome_name: String, validation_data: ValidationData) -> ZomeFnCall {
    let function_name = format!("validate_{}", entry_type.to_string());

    let validation_data_json = serde_json::to_value(
        &validation_data
    ).expect("ValidationData could not be turned into JSON?!");

    let entry_json: serde_json::Value = serde_json::from_str(&*entry).unwrap();
    let params = serde_json::to_string(&json!({
        "entry": entry_json,
        "ctx": validation_data_json,
    })).expect("Params object could not be turned into JSON?!");

    ZomeFnCall::new(
        &zome_name,
        "no capability, since this is an entry validation call",
        &function_name,
        &params,
    )
}



fn run_validation_callback(
    context: Arc<Context>,
    fc: ZomeFnCall,
    wasm: &DnaWasm,
    app_name: String,
) -> CallbackResult {
    match ribosome::api::call(
        &app_name,
        context,
        wasm.code.clone(),
        &fc,
        Some(fc.clone().parameters.into_bytes()),
    ) {
        Ok(runtime) => match runtime.result.is_empty() {
            true => {
                println!("VALIDATION PASSED");
                CallbackResult::Pass
            },
            false => {
                println!("VALIDATION FAILED: {}", runtime.result);
                CallbackResult::Fail(runtime.result)
            },
        },
        Err(err) => {
            println!("VALIDATION ERROR: {}", err);
            CallbackResult::NotImplemented
        },
    }
}



fn get_wasm(context: &Arc<Context>, zome: &str) -> Option<DnaWasm> {
    let dna = get_dna(context).expect("Callback called without DNA set!");
    dna.get_wasm_from_zome_name(zome)
        .and_then(|wasm| {
            if wasm.code.len() > 0 {
                Some(wasm.clone())
            } else {
                None
            }
        })
}
