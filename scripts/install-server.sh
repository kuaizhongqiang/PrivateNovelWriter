#!/usr/bin/env bash
set -euo pipefail

# PNW Server 一键安装脚本
# 适用: Linux (systemd) / macOS (launchd 待补充)
# 用法: sudo bash scripts/install-server.sh

PNW_USER="${PNW_USER:-pnw}"
PNW_DIR="${PNW_DIR:-/opt/pnw}"
PNW_BIN="/usr/local/bin/pnw"

echo "=== PNW Server 安装 ==="

# 1. 检测二进制
if [ ! -f "$PNW_BIN" ]; then
  echo "[1/4] 未找到 $PNW_BIN"
  echo "→ 请先从 GitHub Releases 下载 pnw 二进制到 $PNW_BIN"
  echo "   或手动编译: cargo build --release -p pnw && cp target/release/pnw $PNW_BIN"
  exit 1
fi
chmod +x "$PNW_BIN"
echo "[1/4] ✅ 二进制: $PNW_BIN"

# 2. 创建数据目录
mkdir -p "$PNW_DIR/projects"
echo "[2/4] ✅ 数据目录: $PNW_DIR"

# 3. 配置 .env
if [ ! -f "$PNW_DIR/.env" ]; then
  cat > "$PNW_DIR/.env" << 'ENVEOF'
LLM_PROVIDER=deepseek
LLM_API_KEY=your-api-key-here
LLM_MODEL=deepseek-v4-flash
PNW_PROJECT=/opt/pnw/projects
ENVEOF
  echo "[3/4] ✅ .env 已创建 ($PNW_DIR/.env)"
  echo "    ⚠ 请编辑 $PNW_DIR/.env 填入 LLM_API_KEY"
else
  echo "[3/4] ⏭ .env 已存在: $PNW_DIR/.env"
fi

# 4. 安装 systemd 服务 (仅 Linux)
if [[ "$(uname)" == "Linux" ]]; then
  SERVICE_DIR="/etc/systemd/system"
  SERVICE_FILE="$SERVICE_DIR/pnw-server.service"
  if [ ! -f "$SERVICE_FILE" ]; then
    cp "$(dirname "$0")/pnw-server.service" "$SERVICE_FILE"
    systemctl daemon-reload
    echo "[4/4] ✅ systemd 服务已安装: $SERVICE_FILE"
    echo "    → 启动: systemctl start pnw-server"
    echo "    → 开机自启: systemctl enable pnw-server"
    echo "    → 查看日志: journalctl -u pnw-server -f"
  else
    echo "[4/4] ⏭ systemd 服务已存在: $SERVICE_FILE"
  fi
else
  echo "[4/4] ⏭ 非 Linux 系统，跳过 systemd 配置"
  echo "    → 手动启动: nohup pnw server --host 127.0.0.1 --port 3000 > pnw.log 2>&1 &"
fi

echo ""
echo "=== 安装完成 ==="
echo "项目目录: $PNW_DIR"
echo "创建项目: cd $PNW_DIR/projects && pnw novel new 我的小说"
echo "启动服务: systemctl start pnw-server (Linux)"
echo "API 地址: http://127.0.0.1:3000"
echo "探活: curl http://127.0.0.1:3000/api/health"
