use std::process::Command;

/// E2E test: full CLI workflow
#[test]
fn test_e2e_full_workflow() {
    // Resolve binary path BEFORE changing CWD
    let pnw = binary_path();

    let tmp = std::env::temp_dir().join("pnw_e2e_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    // Use PNW_PROJECT to override project path resolution
    std::env::set_var("PNW_PROJECT", tmp.join("e2e-novel").to_str().unwrap());

    // 1. Create novel
    let out = run(&pnw, &["novel", "new", "e2e-novel"]);
    assert!(out.contains("Created novel"), "create novel: {}", out);

    // 2. Show novel (verify JSON output)
    let out = run(&pnw, &["novel", "show"]);
    assert!(out.contains("e2e-novel"), "show novel: {}", out);

    // 3. Update setting
    let out = run(&pnw, &["setting", "update", "--title", "E2E测试", "--description", "测试"]);
    assert!(!out.contains("Error"), "setting update: {}", out);

    // 4. Add character
    let out = run(&pnw, &["character", "add", "张三", "--char-type", "0", "--age", "25"]);
    assert!(out.contains("Character") || out.contains("张三"), "add char: {}", out);

    // 5. Character list
    let out = run(&pnw, &["character", "list"]);
    assert!(out.contains("张三"), "char list: {}", out);

    // 6. Outline: add phase
    let out = run(&pnw, &["outline", "phase", "add", "第一卷"]);
    assert!(out.contains("Created phase"), "add phase: {}", out);

    // 7. Outline: list phases (get the id)
    let out = run(&pnw, &["outline", "phase", "list"]);
    let pid = extract_json_id(&out);
    assert!(!pid.is_empty(), "no phase id in: {}", out);
    eprintln!("Phase ID: {}", pid);

    // 8. Outline: add chapters
    let out = run(&pnw, &["outline", "chapter", "add", &pid, "第一章", "--content", "剧情开始", "--hook", "悬念"]);
    assert!(out.contains("OutlineChapter"), "add ch1: {}", out);

    let out = run(&pnw, &["outline", "chapter", "add", &pid, "第二章", "--content", "展开", "--hook", "高潮"]);
    assert!(out.contains("OutlineChapter"), "add ch2: {}", out);

    // 9. Outline: list chapters
    let out = run(&pnw, &["outline", "chapter", "list", &pid]);
    assert!(out.contains("OutlineChapterList"), "list outline ch: {}", out);

    // 10. Status
    let out = run(&pnw, &["status"]);
    assert!(out.contains("e2e-novel"), "status: {}", out);

    // 11. Export (even if no chapters, should still succeed)
    let out_path = tmp.join("export.txt");
    let out = run(&pnw, &["export", "txt", "--output", out_path.to_str().unwrap()]);
    assert!(out.contains("导出完成"), "export: {}", out);

    // 12. Sample: add
    let out = run(&pnw, &["sample", "add", "风格参考", "这是样例文字。"]);
    assert!(out.contains("Created sample"), "add sample: {}", out);

    let out = run(&pnw, &["sample", "list"]);
    assert!(out.contains("风格参考"), "list samples: {}", out);

    // Cleanup
    std::env::remove_var("PNW_PROJECT");
    let _ = std::fs::remove_dir_all(&tmp);
}

fn binary_path() -> String {
    let exe = std::env::current_exe().unwrap();
    let target_dir = exe.parent().unwrap()  // deps/
                       .parent().unwrap();   // debug/
    let pnw = target_dir.join(if cfg!(windows) { "pnw.exe" } else { "pnw" });
    pnw.to_str().unwrap().to_string()
}

fn run(binary: &str, args: &[&str]) -> String {
    let output = Command::new(binary)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("exec {} {:?}: {}", binary, args, e));
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!("FAILED: {} {:?}\nstdout: {}\nstderr: {}", binary, args, stdout, stderr);
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Extract a UUID "id" from pretty-printed JSON like:
///   "id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
fn extract_json_id(json: &str) -> String {
    for line in json.lines() {
        let line = line.trim();
        if line.starts_with("\"id\"") {
            // Find the UUID value between quotes
            if let Some(start) = line.find(':') {
                let after = &line[start+1..].trim();
                if after.starts_with('"') {
                    let inner = after.trim_start_matches('"');
                    let id: String = inner.chars().take_while(|c| c != &'"').collect();
                    if id.len() == 36 && id.chars().filter(|&c| c == '-').count() == 4 {
                        return id;
                    }
                }
            }
        }
    }
    String::new()
}
