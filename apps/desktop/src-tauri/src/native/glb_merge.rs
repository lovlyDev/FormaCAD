//! Merge per-body OCCT GLBs without converting the model into JSON mesh arrays.
use crate::native::NativeError;
use serde_json::{json, Value};
use std::{fs, path::Path};

const MAX_BYTES: usize = 40 * 1024 * 1024;
const GLTF_MAGIC: u32 = 0x4654_6c67;
const JSON_CHUNK: u32 = 0x4e4f_534a;
const BIN_CHUNK: u32 = 0x004e_4942;

pub struct BodyGlb<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub path: &'a Path,
}

fn invalid(detail: impl Into<String>) -> NativeError {
    NativeError {
        code: "INVALID_PREVIEW".into(),
        detail: detail.into(),
    }
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, NativeError> {
    let field = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("Truncated GLB header"))?;
    Ok(u32::from_le_bytes(
        field
            .try_into()
            .map_err(|_| invalid("Truncated GLB header"))?,
    ))
}

fn parse_body(path: &Path) -> Result<(Value, Vec<u8>), NativeError> {
    let size = fs::metadata(path)
        .map_err(|error| invalid(error.to_string()))?
        .len();
    if size < 28 || size > MAX_BYTES as u64 {
        return Err(invalid("Per-body GLB has invalid size"));
    }
    let bytes = fs::read(path).map_err(|error| invalid(error.to_string()))?;
    if u32_at(&bytes, 0)? != GLTF_MAGIC
        || u32_at(&bytes, 4)? != 2
        || u32_at(&bytes, 8)? as usize != bytes.len()
        || u32_at(&bytes, 16)? != JSON_CHUNK
    {
        return Err(invalid("Per-body GLB has invalid header"));
    }
    let json_len = u32_at(&bytes, 12)? as usize;
    let binary_header = 20usize
        .checked_add(json_len)
        .ok_or_else(|| invalid("Per-body GLB length overflow"))?;
    if u32_at(&bytes, binary_header + 4)? != BIN_CHUNK {
        return Err(invalid("Per-body GLB binary chunk is missing"));
    }
    let binary_len = u32_at(&bytes, binary_header)? as usize;
    let binary_start = binary_header + 8;
    if binary_start.checked_add(binary_len) != Some(bytes.len()) || !binary_len.is_multiple_of(4) {
        return Err(invalid("Per-body GLB binary chunk is invalid"));
    }
    let document: Value = serde_json::from_slice(
        bytes
            .get(20..binary_header)
            .ok_or_else(|| invalid("Per-body GLB JSON chunk is missing"))?,
    )
    .map_err(|error| invalid(error.to_string()))?;
    if document["meshes"].as_array().is_none_or(|v| v.len() != 1)
        || document["bufferViews"]
            .as_array()
            .is_none_or(|v| v.len() != 2)
        || document["accessors"]
            .as_array()
            .is_none_or(|v| v.len() != 2)
        || document["buffers"][0]["byteLength"].as_u64() != Some(binary_len as u64)
    {
        return Err(invalid("Per-body GLB layout is unsupported"));
    }
    Ok((document, bytes[binary_start..].to_vec()))
}

fn append_u32(output: &mut Vec<u8>, value: usize) -> Result<(), NativeError> {
    let value = u32::try_from(value).map_err(|_| invalid("Combined GLB is too large"))?;
    output.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

pub fn merge_body_glbs(bodies: &[BodyGlb<'_>], destination: &Path) -> Result<(), NativeError> {
    if bodies.is_empty() {
        return Err(invalid("No body GLBs to merge"));
    }
    let mut nodes = Vec::with_capacity(bodies.len());
    let mut meshes = Vec::with_capacity(bodies.len());
    let mut views = Vec::with_capacity(bodies.len() * 2);
    let mut accessors = Vec::with_capacity(bodies.len() * 2);
    let mut binary = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        let (document, chunk) = parse_body(body.path)?;
        if binary
            .len()
            .checked_add(chunk.len())
            .is_none_or(|len| len > MAX_BYTES)
        {
            return Err(invalid("Combined GLB exceeds 40 MB"));
        }
        let view_offset = views.len();
        let accessor_offset = accessors.len();
        let mut mesh = document["meshes"][0].clone();
        mesh["primitives"][0]["attributes"]["POSITION"] = json!(accessor_offset);
        mesh["primitives"][0]["indices"] = json!(accessor_offset + 1);
        meshes.push(mesh);
        let body_views = document["bufferViews"]
            .as_array()
            .ok_or_else(|| invalid("GLB buffer views are missing"))?;
        for (position, source) in body_views.iter().enumerate() {
            let mut view = source.clone();
            let offset = source["byteOffset"]
                .as_u64()
                .ok_or_else(|| invalid("GLB buffer view has no offset"))?;
            let shifted = offset
                .checked_add(binary.len() as u64)
                .ok_or_else(|| invalid("GLB buffer offset overflow"))?;
            view["byteOffset"] = json!(shifted);
            view["buffer"] = json!(0);
            views.push(view);
            let mut accessor = document["accessors"][position].clone();
            accessor["bufferView"] = json!(view_offset + position);
            accessors.push(accessor);
        }
        nodes.push(json!({
            "mesh": index,
            "name": body.id,
            "extras": {"formaBodyId": body.id, "formaDisplayName": body.name}
        }));
        binary.extend_from_slice(&chunk);
    }
    let scene_nodes: Vec<usize> = (0..bodies.len()).collect();
    let gltf = json!({
        "asset": {"version": "2.0", "generator": "Forma CAD"},
        "scene": 0,
        "scenes": [{"nodes": scene_nodes}],
        "nodes": nodes,
        "meshes": meshes,
        "buffers": [{"byteLength": binary.len()}],
        "bufferViews": views,
        "accessors": accessors,
    });
    let mut json_bytes = serde_json::to_vec(&gltf).map_err(|error| invalid(error.to_string()))?;
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    let total = 28usize
        .checked_add(json_bytes.len())
        .and_then(|value| value.checked_add(binary.len()))
        .ok_or_else(|| invalid("Combined GLB length overflow"))?;
    if total > MAX_BYTES {
        return Err(invalid("Combined GLB exceeds 40 MB"));
    }
    let mut output = Vec::with_capacity(total);
    append_u32(&mut output, GLTF_MAGIC as usize)?;
    append_u32(&mut output, 2)?;
    append_u32(&mut output, total)?;
    append_u32(&mut output, json_bytes.len())?;
    append_u32(&mut output, JSON_CHUNK as usize)?;
    output.extend_from_slice(&json_bytes);
    append_u32(&mut output, binary.len())?;
    append_u32(&mut output, BIN_CHUNK as usize)?;
    output.extend_from_slice(&binary);
    fs::write(destination, output).map_err(|error| invalid(error.to_string()))
}
