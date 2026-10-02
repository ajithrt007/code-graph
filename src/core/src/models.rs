use std::path::PathBuf;

struct LineRange {
    start: i32,
    end: i32,
}

pub struct Node {
    id: String,
    is_inside_class: bool,
    method_name: String,
    full_name: String,
    code_lines: String,
    file_path: PathBuf,
    line: LineRange,
}

pub struct Edge {
    from: String,
    to: String,
}