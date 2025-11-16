// ============================================================
// AON CORE — v1.3 (JSON <-> AON estável, com listas e subs)
// ============================================================

use lazy_static::lazy_static;
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

/* ============================ *
 * ERROR HANDLING (GLOBAL)
 * ============================ */

lazy_static! {
    static ref LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
}

fn set_error(msg: impl Into<String>) {
    *LAST_ERROR.lock().unwrap() = Some(msg.into());
}

fn clear_error() {
    *LAST_ERROR.lock().unwrap() = None;
}

#[no_mangle]
pub extern "C" fn aon_last_error() -> *const c_char {
    let guard = LAST_ERROR.lock().unwrap();
    if let Some(err) = &*guard {
        if let Ok(c) = CString::new(err.clone()) {
            return c.into_raw();
        }
    }
    std::ptr::null()
}

#[no_mangle]
pub extern "C" fn aon_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { let _ = CString::from_raw(ptr); }
    }
}

/* ============================ *
 * TYPE INFERENCE
 * ============================ */

fn infer_type(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/* ============================== *
 * SCHEMA DEFINITIONS
 * ============================== */

#[derive(Debug, Clone)]
struct SchemaField {
    name: String,
    ty: String,
}

#[derive(Debug, Clone)]
struct Schema {
    name: String,
    fields: Vec<SchemaField>,
}

/* ============================== *
 * SCHEMA INFERENCE REFATORADO
 * ============================== */

/// Coleta objetos em um caminho (path) dentro do JSON:
/// ex: path ["profile", "enderecos"] pega todos os objetos dentro desse array.
fn collect_objects_from_path(root: &Value, path: &[String]) -> Vec<Value> {
    if path.is_empty() {
        return match root {
            Value::Array(arr) => arr.clone(),
            Value::Object(_) => vec![root.clone()],
            _ => vec![],
        };
    }

    let key = &path[0];
    let rest = &path[1..];

    match root {
        Value::Array(arr) => {
            let mut results = Vec::new();
            for item in arr {
                if let Value::Object(map) = item {
                    if let Some(child) = map.get(key) {
                        results.extend(collect_objects_from_path(child, rest));
                    }
                }
            }
            results
        }
        Value::Object(map) => {
            if let Some(child) = map.get(key) {
                collect_objects_from_path(child, rest)
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

/// Constrói todos os schemas globais a partir do JSON + root_name
fn build_all_schemas(value: &Value, root_name: &str) -> HashMap<String, Schema> {
    let mut schemas = HashMap::new();
    let mut to_process: Vec<(String, Vec<String>, Value)> =
        vec![(root_name.to_string(), vec![], value.clone())];

    while let Some((schema_name, path, sample)) = to_process.pop() {
        // Coletar todos os objetos desse tipo
        let objects = if path.is_empty() {
            match &sample {
                Value::Array(arr) => arr.clone(),
                Value::Object(_) => vec![sample.clone()],
                _ => continue,
            }
        } else {
            collect_objects_from_path(value, &path)
        };

        if objects.is_empty() {
            continue;
        }

        // Extrair campos únicos
        let mut all_fields: HashMap<String, Vec<Value>> = HashMap::new();

        for obj in &objects {
            if let Value::Object(map) = obj {
                for (key, val) in map {
                    all_fields
                        .entry(key.clone())
                        .or_insert_with(Vec::new)
                        .push(val.clone());
                }
            }
        }

        // Criar schema
        let mut fields = Vec::new();

        for (field_name, values) in all_fields {
            let field_type = infer_field_type(&field_name, &values, &mut to_process, &path);
            fields.push(SchemaField {
                name: field_name,
                ty: field_type,
            });
        }

        let schema_name_clone = schema_name.clone();
        schemas.insert(
            schema_name,
            Schema {
                name: schema_name_clone,
                fields,
            },
        );
    }

    schemas
}

fn infer_field_type(
    field_name: &str,
    values: &[Value],
    to_process: &mut Vec<(String, Vec<String>, Value)>,
    parent_path: &[String],
) -> String {
    // Objeto direto (subschema)
    let has_objects = values.iter().any(|v| matches!(v, Value::Object(_)));
    if has_objects {
        let mut new_path = parent_path.to_vec();
        new_path.push(field_name.to_string());

        if let Some(first_obj) = values.iter().find(|v| matches!(v, Value::Object(_))) {
            to_process.push((field_name.to_string(), new_path, first_obj.clone()));
        }

        return field_name.to_string();
    }

    // Array
    let has_arrays = values.iter().any(|v| matches!(v, Value::Array(_)));
    if has_arrays {
        let mut has_array_objects = false;
        for val in values {
            if let Value::Array(arr) = val {
                if arr.iter().any(|v| matches!(v, Value::Object(_))) {
                    has_array_objects = true;
                    break;
                }
            }
        }

        if has_array_objects {
            // list<subschema>
            let mut new_path = parent_path.to_vec();
            new_path.push(field_name.to_string());

            for val in values {
                if let Value::Array(arr) = val {
                    if let Some(first_obj) = arr.iter().find(|v| matches!(v, Value::Object(_))) {
                        to_process.push((field_name.to_string(), new_path.clone(), first_obj.clone()));
                        break;
                    }
                }
            }

            return format!("list<{}>", field_name);
        }

        // lista simples (sem objeto)
        return "list<string>".to_string();
    }

    // Primitivo
    for val in values {
        if !val.is_null() {
            return infer_type(val).to_string();
        }
    }

    "null".to_string()
}

/* ============================== *
 * DATA ENCODING REFATORADO
 * ============================== */

fn encode_value(v: &Value) -> String {
    match v {
        Value::Null => "_".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            // CEP e similares podem ser só número puro
            if s.chars().all(|c| c.is_ascii_digit()) {
                s.clone()
            } else {
                format!("{:?}", s)
            }
        }
        _ => "_".to_string(),
    }
}

fn encode_object_with_schema(
    obj: &Value,
    schema: &Schema,
    all_schemas: &HashMap<String, Schema>,
) -> String {
    if let Value::Object(map) = obj {
        let mut parts = Vec::new();

        for field in &schema.fields {
            let val = map.get(&field.name).unwrap_or(&Value::Null);

            // Subobjeto
            if field.ty == field.name {
                if let Some(sub_schema) = all_schemas.get(&field.name) {
                    parts.push(format!(
                        "({})",
                        encode_object_with_schema(val, sub_schema, all_schemas)
                    ));
                } else {
                    parts.push("_".to_string());
                }
            }
            // Lista
            else if field.ty.starts_with("list<") && field.ty.ends_with('>') {
                let inner = &field.ty[5..field.ty.len() - 1];

                if let Some(sub_schema) = all_schemas.get(inner) {
                    // lista de subschema
                    if let Value::Array(arr) = val {
                        let items: Vec<String> = arr
                            .iter()
                            .map(|item| {
                                format!(
                                    "({})",
                                    encode_object_with_schema(item, sub_schema, all_schemas)
                                )
                            })
                            .collect();
                        parts.push(format!("[{}]", items.join(" ; ")));
                    } else {
                        parts.push("[]".to_string());
                    }
                } else if let Value::Array(arr) = val {
                    // lista simples
                    let items: Vec<String> =
                        arr.iter().map(|item| encode_value(item)).collect();
                    parts.push(format!("[{}]", items.join(" ; ")));
                } else {
                    parts.push("[]".to_string());
                }
            }
            // Primitivo
            else {
                parts.push(encode_value(val));
            }
        }

        parts.join(",")
    } else {
        "_".to_string()
    }
}

/* ============================== *
 * MAIN JSON → AON
 * ============================== */

fn json_to_aon(json: &Value, root_name: &str) -> Result<String, String> {
    let rows = match json {
        Value::Array(arr) => arr.clone(),
        Value::Object(_) => vec![json.clone()],
        _ => return Err("JSON root must be array or object".into()),
    };

    if rows.is_empty() {
        return Err("Empty JSON".into());
    }

    let schemas = build_all_schemas(json, root_name);

    if !schemas.contains_key(root_name) {
        return Err(format!("Root schema '{}' not found", root_name));
    }

    let mut out = String::new();
    out.push_str("!aon\n");
    out.push_str(&format!("count:{}\n", rows.len()));
    out.push_str("schemas:{\n");

    // root schema primeiro, depois os outros
    let mut schema_entries: Vec<_> = schemas.iter().collect();
    schema_entries.sort_by_key(|(name, _)| {
        if *name == root_name {
            0
        } else {
            1
        }
    });

    for (name, schema) in schema_entries {
        let fields: Vec<String> = schema
            .fields
            .iter()
            .map(|f| format!("{}:{}", f.name, f.ty))
            .collect();
        out.push_str(&format!("  {}:({})\n", name, fields.join(",")));
    }

    out.push_str("}\n");
    out.push_str("data:\n");

    let root_schema = &schemas[root_name];
    for row in &rows {
        out.push_str(&encode_object_with_schema(row, root_schema, &schemas));
        out.push('\n');
    }

    out.push_str("end\n");
    Ok(out)
}

/* ============================== *
 * AON PARSER (SIMPLES E TIPADO)
 * ============================== */

fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut depth_paren = 0;
    let mut depth_brack = 0;

    for ch in s.chars() {
        match ch {
            '(' => {
                depth_paren += 1;
                buf.push(ch);
            }
            ')' => {
                depth_paren -= 1;
                buf.push(ch);
            }
            '[' => {
                depth_brack += 1;
                buf.push(ch);
            }
            ']' => {
                depth_brack -= 1;
                buf.push(ch);
            }
            ',' if depth_paren == 0 && depth_brack == 0 => {
                parts.push(buf.trim().to_string());
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }

    if !buf.trim().is_empty() {
        parts.push(buf.trim().to_string());
    }

    parts
}

fn split_top_level_semicolons(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut buf = String::new();
    let mut depth_paren = 0;
    let mut depth_brack = 0;

    for ch in s.chars() {
        match ch {
            '(' => {
                depth_paren += 1;
                buf.push(ch);
            }
            ')' => {
                depth_paren -= 1;
                buf.push(ch);
            }
            '[' => {
                depth_brack += 1;
                buf.push(ch);
            }
            ']' => {
                depth_brack -= 1;
                buf.push(ch);
            }
            ';' if depth_paren == 0 && depth_brack == 0 => {
                parts.push(buf.trim().to_string());
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }

    if !buf.trim().is_empty() {
        parts.push(buf.trim().to_string());
    }

    parts
}

fn decode_value(
    ty: &str,
    s: &str,
    schemas: &HashMap<String, Vec<(String, String)>>,
) -> Value {
    let s = s.trim();

    // null compacto
    if s == "_" {
        return Value::Null;
    }

    // primitivos
    if ty == "boolean" {
        return Value::Bool(s == "true");
    }

    if ty == "number" {
        if let Ok(i) = s.parse::<i64>() {
            return Value::Number(i.into());
        }
        if let Ok(f) = s.parse::<f64>() {
            if let Some(n) = serde_json::Number::from_f64(f) {
                return Value::Number(n);
            }
        }
        return Value::Null;
    }

    if ty == "string" {
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            return Value::String(s[1..s.len() - 1].to_string());
        } else {
            return Value::String(s.to_string());
        }
    }

    // lista<T>
    if ty.starts_with("list<") && ty.ends_with('>') {
        let inner_ty = &ty[5..ty.len() - 1];
        if !s.starts_with('[') || !s.ends_with(']') {
            return Value::Array(vec![]);
        }

        let inner = &s[1..s.len() - 1].trim();
        if inner.is_empty() {
            return Value::Array(vec![]);
        }

        let items_str = split_top_level_semicolons(inner);
        let mut items = Vec::new();
        for item_s in items_str {
            items.push(decode_value(inner_ty, &item_s, schemas));
        }
        return Value::Array(items);
    }

    // subschema (tipo == nome do schema)
    if let Some(fields) = schemas.get(ty) {
        if !s.starts_with('(') || !s.ends_with(')') {
            return Value::Null;
        }

        let inner = &s[1..s.len() - 1];
        let parts = split_top_level_commas(inner);

        let mut map = serde_json::Map::new();
        for ((fname, fty), part_s) in fields.iter().zip(parts.iter()) {
            let v = decode_value(fty, part_s, schemas);
            map.insert(fname.clone(), v);
        }

        return Value::Object(map);
    }

    // fallback: string
    Value::String(s.to_string())
}

fn aon_to_json(aon: &str) -> Result<Value, String> {
    let mut schemas: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut data_rows: Vec<String> = Vec::new();
    let mut root_name = String::new();

    let mut in_schemas = false;
    let mut in_data = false;

    for line in aon.lines() {
        let line = line.trim();

        if line == "schemas:{" {
            in_schemas = true;
            continue;
        }
        if line == "}" {
            in_schemas = false;
            continue;
        }
        if line == "data:" {
            in_data = true;
            continue;
        }
        if line == "end" {
            break;
        }

        if in_schemas && line.contains(":(") {
            let pos = line.find(":(").unwrap();
            let name = line[..pos].trim().to_string();
            let fields_str = &line[pos + 2..line.len() - 1];

            let mut fields = Vec::new();
            for field_def in fields_str.split(',') {
                if let Some(colon) = field_def.find(':') {
                    let fname = field_def[..colon].trim().to_string();
                    let ftype = field_def[colon + 1..].trim().to_string();
                    fields.push((fname, ftype));
                }
            }

            if root_name.is_empty() {
                root_name = name.clone();
            }

            schemas.insert(name, fields);
        }

        if in_data && !line.is_empty() {
            data_rows.push(line.to_string());
        }
    }

    if root_name.is_empty() {
        return Err("No schemas found".into());
    }

    let root_schema = schemas
        .get(&root_name)
        .ok_or_else(|| "Root schema not found".to_string())?;

    let mut results = Vec::new();

    for row in data_rows {
        let parts = split_top_level_commas(&row);

        if parts.len() != root_schema.len() {
            return Err(format!(
                "Data row size mismatch: expected {}, got {}",
                root_schema.len(),
                parts.len()
            ));
        }

        let mut obj = serde_json::Map::new();

        for ((fname, fty), part_s) in root_schema.iter().zip(parts.iter()) {
            let v = decode_value(fty, part_s, &schemas);
            obj.insert(fname.clone(), v);
        }

        results.push(Value::Object(obj));
    }

    if results.len() == 1 {
        Ok(results.into_iter().next().unwrap())
    } else {
        Ok(Value::Array(results))
    }
}

/* ============================== *
 * FFI EXPORTS
 * ============================== */

#[no_mangle]
pub extern "C" fn aon_json_to_aon(
    json_c: *const c_char,
    root_c: *const c_char,
) -> *mut c_char {
    clear_error();

    if json_c.is_null() || root_c.is_null() {
        set_error("null pointer");
        return std::ptr::null_mut();
    }

    let json_str = unsafe { CStr::from_ptr(json_c) }.to_str().unwrap();
    let root_str = unsafe { CStr::from_ptr(root_c) }.to_str().unwrap();

    let json_val: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(e.to_string());
            return std::ptr::null_mut();
        }
    };

    match json_to_aon(&json_val, root_str) {
        Ok(aon) => CString::new(aon).unwrap().into_raw(),
        Err(e) => {
            set_error(e);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn aon_aon_to_json(aon_c: *const c_char) -> *mut c_char {
    clear_error();

    if aon_c.is_null() {
        set_error("null pointer");
        return std::ptr::null_mut();
    }

    let aon_str = unsafe { CStr::from_ptr(aon_c) }.to_str().unwrap();

    match aon_to_json(aon_str) {
        Ok(v) => {
            let js = serde_json::to_string(&v).unwrap();
            CString::new(js).unwrap().into_raw()
        }
        Err(e) => {
            set_error(e);
            std::ptr::null_mut()
        }
    }
}
