# conduit-web-demo

单二进制部署的 Conduit 演示:一个 Web 管理界面(增删服务器 / token)+ 一个**实活的 MCP 端点**,共用同一个 SQLite 库。在浏览器里配好服务器和 token,AI 客户端就能直接连 `/mcp` 操作这些服务器。

```
┌─────────────── conduit-web-demo (单二进制) ───────────────┐
│  /            内嵌的管理 UI(原生 JS,无构建步骤)          │
│  /api/*       管理 API(增删服务器/token,admin 密码网关)  │
│  /mcp         conduit MCP 端点(Bearer = UI 签发的 token)  │
└───────────────────────┬───────────────────────────────────┘
                         │ 同一个 SQLite 库 + 同一把 master key
        写:demo 自带的 store     读:conduit-store-dibs(引擎适配器)
```

- **访问模型(简化版)**:单一隐式用户,所有 token 都能访问所有服务器。
- **凭据安全**:密码/私钥/证书用 ChaCha20-Poly1305 加密入库;token 只存 sha256 哈希,明文仅创建时显示一次。AI 永远拿不到凭据,只拿不透明 `session_id`。

## 运行

```bash
cargo run -- --bind 127.0.0.1:8088 --db ./conduit-demo.db
```

- 不带 `--master-key` 时,自动在 `<db>.key` 生成并复用一把(请妥善保管,丢了库里凭据就解不开)。
- 不带 `--admin-password` 时,启动日志会打印一个随机管理密码。
- 默认只监听 `127.0.0.1`。要对外暴露,显式设 `--bind 0.0.0.0:8088`(或前置反向代理)。

打开 `http://127.0.0.1:8088/`,输入管理密码 → 添加服务器、创建 token。

## 配置

| 参数 | 环境变量 | 默认 | 说明 |
|------|---------|------|------|
| `--bind` | `CONDUIT_WEB_BIND` | `127.0.0.1:8088` | Web + MCP 监听地址 |
| `--db` | `CONDUIT_DB` | `./conduit-demo.db` | SQLite 路径 |
| `--master-key` | `CONDUIT_MASTER_KEY` | (自动生成 keyfile) | 64-hex 加密主密钥 |
| `--admin-password` | `CONDUIT_ADMIN_PASSWORD` | (自动随机) | 管理 UI 密码 |
| `--rate-per-min` | `CONDUIT_RATE_PER_MIN` | 30 | 每 token 每分钟命令数 |
| `--idle-timeout-secs` | `CONDUIT_IDLE_TIMEOUT_SECS` | 1800 | 空闲会话回收秒数 |

## MCP 客户端接入

把任意 MCP 客户端指向 `http://<bind>/mcp`,带上 UI 里创建的 token:

```
Authorization: Bearer cdt_xxxxxxxx...
```

可用工具:`list_servers` · `open_channel` · `exec` · `exec_start`/`exec_poll`/`exec_stop`(长任务监控)· `sftp_list`/`sftp_download`/`sftp_upload` · `close_channel`。

## 依赖说明

通过 path 依赖引用同目录旁的 `../conduit`(`conduit-core` / `conduit-engine` / `conduit-store-dibs`)。要改成从 GitHub 拉,把 `Cargo.toml` 里的 path 换成:

```toml
conduit-engine = { git = "https://github.com/Everless321/conduit", package = "conduit-engine" }
```
