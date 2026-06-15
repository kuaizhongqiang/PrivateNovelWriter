use std::io::Read;
use std::process::Command;

fn test_dir() -> std::path::PathBuf {
    // On Windows CI, temp_dir may return 8.3 short paths (RUNNER~1) that confuse SQLite.
    // Use GITHUB_WORKSPACE (D:\a\...) which has no short name.
    if let Ok(ws) = std::env::var("GITHUB_WORKSPACE") {
        std::path::PathBuf::from(ws)
    } else {
        std::env::temp_dir()
    }
}

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

/// Server mode integration test — disabled in CI due to Windows 8.3 path issues
#[test]
#[ignore]
fn test_server_integration() {
    let pnw = server_binary_path();
    let tmp = test_dir().join("pnw_server_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_current_dir(&tmp).unwrap();

    let out = run(&pnw, &["novel", "new", "server-test"]);
    assert!(out.contains("Created novel"), "create novel: {}", out);

    let port = 19191 + (std::process::id() % 1000) as u16;
    // canonicalize after project exists to resolve Windows 8.3 short paths
    let proj_path =
        std::fs::canonicalize(tmp.join("server-test")).unwrap_or(tmp.join("server-test"));
    let mut server = Command::new(&pnw)
        .args([
            "server",
            "--port",
            &port.to_string(),
            "--project",
            proj_path.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    std::thread::sleep(std::time::Duration::from_millis(500));
    if let Ok(Some(status)) = server.try_wait() {
        let mut err_buf = String::new();
        if let Some(mut stderr) = server.stderr.take() {
            stderr.read_to_string(&mut err_buf).ok();
        }
        panic!("Server exited early with code {}:\n{}", status, err_buf);
    }
    wait_for_server(port);
    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    let get = |path: &str| -> serde_json::Value {
        for i in 0..10 {
            let resp = client.get(format!("{}{}", base, path)).send();
            match resp {
                Ok(r) => {
                    let body = r.json::<serde_json::Value>();
                    if let Ok(v) = body {
                        return v;
                    }
                    eprintln!("[retry {}] json parse failed for {}", i, path);
                }
                Err(e) => {
                    eprintln!("[retry {}] send failed for {}: {}", i, path, e);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        panic!("Failed to GET {} after 10 retries", path);
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

    // Create outline phase + chapter (test sort serialization #118)
    let outline_phase = client
        .post(format!("{}/api/command", base))
        .json(&serde_json::json!({"command": "create_outline_phase", "args": {"name": "第一卷"}}))
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    assert_eq!(outline_phase["status"], "ok");
    let phase_id = outline_phase["data"]["OutlinePhase"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let ch1 = client
        .post(format!("{}/api/command", base))
        .json(&serde_json::json!({"command": "create_outline_chapter", "args": {"phase_id": phase_id, "name": "第一章", "content": "开始", "hook": "悬念"}}))
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    assert_eq!(ch1["status"], "ok");
    assert_eq!(ch1["data"]["OutlineChapter"]["sort"], 0);

    // Create text phase + chapter via API
    let text_phase = client
        .post(format!("{}/api/command", base))
        .json(&serde_json::json!({"command": "create_text_phase", "args": {"name": "第一卷"}}))
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    assert_eq!(text_phase["status"], "ok");
    let text_phase_id = text_phase["data"]["TextPhase"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let text_ch = client
        .post(format!("{}/api/command", base))
        .json(&serde_json::json!({"command": "create_text_chapter", "args": {"phase_id": text_phase_id, "name": "第一章"}}))
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    assert_eq!(text_ch["status"], "ok");
    let tc_id = text_ch["data"]["TextChapter"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Test GET /api/chapter/{id} (#116/#119)
    let r = client
        .get(format!("{}/api/chapter/{}", base, tc_id))
        .send()
        .unwrap();
    assert!(
        r.status().is_success(),
        "GET /api/chapter/{{id}} returned {}",
        r.status()
    );
    let body: serde_json::Value = r.json().unwrap();
    assert_eq!(body["status"], "ok", "chapter get: {:?}", body);
    assert_eq!(body["data"]["id"], tc_id);
    assert!(body["data"]["phase_name"].as_str().is_some());

    // Test PUT /api/chapter/{id} (save)
    let save = client
        .put(format!("{}/api/chapter/{}", base, tc_id))
        .json(&serde_json::json!({"content": "测试正文内容"}))
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    assert_eq!(save["status"], "ok");

    // Test GET after save
    let r2 = client
        .get(format!("{}/api/chapter/{}", base, tc_id))
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    assert_eq!(r2["status"], "ok");
    assert_eq!(r2["data"]["content"], "测试正文内容");

    // Test sort ordering (#118 fix verification)
    let ch2 = client
        .post(format!("{}/api/command", base))
        .json(&serde_json::json!({"command": "create_outline_chapter", "args": {"phase_id": phase_id, "name": "第二章"}}))
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    assert_eq!(ch2["status"], "ok");
    assert_eq!(
        ch2["data"]["OutlineChapter"]["sort"], 1,
        "sort should increment: {:?}",
        ch2
    );

    // Test project switch (#117)
    let project2 = test_dir().join("pnw_server_test_2");
    let _ = std::fs::remove_dir_all(&project2);
    std::fs::create_dir_all(&project2).unwrap();
    let out = run(&pnw, &["novel", "new", "project2"]);
    assert!(out.contains("Created novel"), "create project2: {}", out);
    let proj2_path =
        std::fs::canonicalize(project2.join("project2")).unwrap_or(project2.join("project2"));
    let switch = client
        .post(format!("{}/api/project/switch", base))
        .json(&serde_json::json!({"path": proj2_path.to_str().unwrap()}))
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();
    assert_eq!(switch["status"], "ok", "switch project: {:?}", switch);
    // Confirm we're on the new project
    let proj = get("/api/project");
    assert_eq!(proj["status"], "ok");
    assert_eq!(proj["data"]["name"], "project2");

    // Gateway UI
    let html = client.get(base.to_string()).send().unwrap().text().unwrap();
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
    let tmp = test_dir().join("pnw_e2e_test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_current_dir(&tmp).unwrap();

    let out = run(&pnw, &["novel", "new", "e2e-novel"]);
    assert!(out.contains("Created novel"), "create novel: {}", out);
    // cd into project dir so get_project_path() finds project.db in CWD
    std::env::set_current_dir(tmp.join("e2e-novel")).unwrap();
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
