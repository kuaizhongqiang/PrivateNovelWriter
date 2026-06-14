use std::process::Command;

fn server_binary_path() -> String {
    let exe = std::env::current_exe().unwrap();
    let target_dir = exe.parent().unwrap().parent().unwrap();
    let pnw = target_dir.join(if cfg!(windows) { "pnw.exe" } else { "pnw" });
    pnw.to_str().unwrap().to_string()
}

fn wait_for_server(port: u16) {
    let url = format!("http://127.0.0.1:{}/api/status", port);
    for _ in 0..30 {
        if let Ok(resp) = reqwest::blocking::get(&url) {
            if resp.status().is_success() {
                return;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    match reqwest::blocking::get(&url) {
        Ok(resp) => panic!("Server returned HTTP {}", resp.status()),
        Err(e) => panic!("Server did not start: {}", e),
    }
}

/// Server mode integration test
#[test]
fn test_server_integration() {
    let pnw = server_binary_path();
    let tmp = std::env::temp_dir().join("pnw_server_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_current_dir(&tmp).unwrap();

    let out = run(&pnw, &["novel", "new", "server-test"]);
    assert!(out.contains("Created novel"), "create novel: {}", out);

    let port = 19191 + (std::process::id() % 1000) as u16;
    let mut server = Command::new(&pnw)
        .args([
            "server",
            "--port",
            &port.to_string(),
            "--project",
            tmp.join("server-test").to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Failed to start server");

    std::thread::sleep(std::time::Duration::from_millis(500));
    if let Ok(Some(status)) = server.try_wait() {
        panic!("Server exited early with code {}", status);
    }
    wait_for_server(port);
    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::blocking::Client::new();
    let get = |path: &str| -> serde_json::Value {
        client
            .get(format!("{}{}", base, path))
            .send()
            .unwrap()
            .json()
            .unwrap()
    };

    // All GET endpoints
    assert_eq!(get("/api/status")["status"], "ok");
    assert!(get("/api/status")["session_start"].as_u64().unwrap_or(0) > 0);
    assert_eq!(get("/api/project")["status"], "ok");
    assert_eq!(get("/api/outline")["status"], "ok");
    assert_eq!(get("/api/stats")["status"], "ok");
    assert_eq!(get("/api/characters")["status"], "ok");
    let s = get("/api/setting");
    assert!(s["status"] == "ok" || s["status"] == "error");
    assert_eq!(get("/api/samples")["status"], "ok");
    assert_eq!(get("/api/chapters")["status"], "ok");
    let e = get("/api/export/txt");
    assert_eq!(e["status"], "ok");
    assert!(e["chapter_count"].as_u64().is_some());

    // POST /api/command
    assert_eq!(
        client
            .post(format!("{}/api/command", base))
            .json(&serde_json::json!({"command": "get_novel"}))
            .send()
            .unwrap()
            .json::<serde_json::Value>()
            .unwrap()["status"],
        "ok"
    );
    assert_eq!(
        client
            .post(format!("{}/api/command", base))
            .json(&serde_json::json!({"command": "create_character", "args": {"name": "测试角色"}}))
            .send()
            .unwrap()
            .json::<serde_json::Value>()
            .unwrap()["status"],
        "ok"
    );
    assert_eq!(
        client
            .post(format!("{}/api/command", base))
            .json(&serde_json::json!({"command": "nonexistent"}))
            .send()
            .unwrap()
            .json::<serde_json::Value>()
            .unwrap()["status"],
        "error"
    );

    // Gateway UI
    let html = client
        .get(base.to_string())
        .send()
        .unwrap()
        .text()
        .unwrap();
    assert!(html.contains("PNW Gateway"));

    // Cleanup
    let _ = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &server.id().to_string()])
        .output();
    std::env::set_current_dir(std::env::temp_dir()).ok();
    let _ = std::fs::remove_dir_all(&tmp);
}

/// E2E test: full CLI workflow
#[test]
fn test_e2e_full_workflow() {
    let pnw = binary_path();
    let tmp = std::env::temp_dir().join("pnw_e2e_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    std::env::set_var("PNW_PROJECT", tmp.join("e2e-novel").to_str().unwrap());

    let out = run(&pnw, &["novel", "new", "e2e-novel"]);
    assert!(out.contains("Created novel"), "create novel: {}", out);
    let out = run(&pnw, &["novel", "show"]);
    assert!(out.contains("e2e-novel"), "show novel: {}", out);
    let out = run(
        &pnw,
        &[
            "setting",
            "update",
            "--title",
            "E2E测试",
            "--description",
            "测试",
        ],
    );
    assert!(!out.contains("Error"), "setting update: {}", out);
    let out = run(
        &pnw,
        &[
            "character",
            "add",
            "张三",
            "--char-type",
            "0",
            "--age",
            "25",
        ],
    );
    assert!(
        out.contains("Character") || out.contains("张三"),
        "add char: {}",
        out
    );
    let out = run(&pnw, &["character", "list"]);
    assert!(out.contains("张三"), "char list: {}", out);
    let out = run(&pnw, &["outline", "phase", "add", "第一卷"]);
    assert!(out.contains("Created phase"), "add phase: {}", out);
    let out = run(&pnw, &["outline", "phase", "list"]);
    let pid = extract_json_id(&out);
    assert!(!pid.is_empty(), "no phase id in: {}", out);
    let out = run(
        &pnw,
        &[
            "outline",
            "chapter",
            "add",
            &pid,
            "第一章",
            "--content",
            "剧情开始",
            "--hook",
            "悬念",
        ],
    );
    assert!(out.contains("OutlineChapter"), "add ch1: {}", out);
    let out = run(
        &pnw,
        &[
            "outline",
            "chapter",
            "add",
            &pid,
            "第二章",
            "--content",
            "展开",
            "--hook",
            "高潮",
        ],
    );
    assert!(out.contains("OutlineChapter"), "add ch2: {}", out);
    let out = run(&pnw, &["outline", "chapter", "list", &pid]);
    assert!(
        out.contains("OutlineChapterList"),
        "list outline ch: {}",
        out
    );
    let out = run(&pnw, &["status"]);
    assert!(out.contains("e2e-novel"), "status: {}", out);
    let out_path = tmp.join("export.txt");
    let out = run(
        &pnw,
        &["export", "txt", "--output", out_path.to_str().unwrap()],
    );
    assert!(out.contains("导出完成"), "export: {}", out);
    let out = run(&pnw, &["sample", "add", "风格参考", "这是样例文字。"]);
    assert!(out.contains("Created sample"), "add sample: {}", out);
    let out = run(&pnw, &["sample", "list"]);
    assert!(out.contains("风格参考"), "list samples: {}", out);

    std::env::remove_var("PNW_PROJECT");
    let _ = std::fs::remove_dir_all(&tmp);
}

fn binary_path() -> String {
    let exe = std::env::current_exe().unwrap();
    let target_dir = exe.parent().unwrap().parent().unwrap();
    let pnw = target_dir.join(if cfg!(windows) { "pnw.exe" } else { "pnw" });
    pnw.to_str().unwrap().to_string()
}

fn run(binary: &str, args: &[&str]) -> String {
    let output = Command::new(binary)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("exec {} {:?}: {}", binary, args, e));
    if !output.status.success() {
        panic!(
            "FAILED: {} {:?}\nstderr: {}",
            binary,
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn extract_json_id(json: &str) -> String {
    for line in json.lines() {
        let line = line.trim();
        if line.starts_with("\"id\"") {
            if let Some(start) = line.find(':') {
                let after = line[start + 1..].trim();
                if after.starts_with('"') {
                    let id: String = after
                        .trim_start_matches('"')
                        .chars()
                        .take_while(|c| c != &'"')
                        .collect();
                    if id.len() == 36 && id.chars().filter(|&c| c == '-').count() == 4 {
                        return id;
                    }
                }
            }
        }
    }
    String::new()
}
