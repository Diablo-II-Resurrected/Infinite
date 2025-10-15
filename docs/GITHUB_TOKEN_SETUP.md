# GitHub Token 配置指南

## 为什么需要 GitHub Token?

GitHub API 对未认证请求有严格的速率限制:

- **未认证**: 60 请求/小时 (按 IP 地址)
- **已认证**: 5000 请求/小时 (按用户)

如果你频繁使用 GitHub mod 功能,强烈建议配置 Personal Access Token。

## 创建 GitHub Personal Access Token

### 步骤 1: 访问 GitHub 设置

1. 登录 GitHub
2. 访问: https://github.com/settings/tokens
3. 或者: `Settings` → `Developer settings` → `Personal access tokens` → `Tokens (classic)`

### 步骤 2: 生成新 Token

1. 点击 **"Generate new token"** → **"Generate new token (classic)"**
2. 给 token 起个名字,例如: `Infinite Mod Manager`
3. 设置过期时间 (建议选择 `No expiration` 或较长时间)

### 步骤 3: 选择权限

**最小权限** - 只需勾选:
- ✅ `public_repo` (Access public repositories)

这是读取公开仓库所需的唯一权限,不会授予任何写入或私有访问权限。

### 步骤 4: 生成并保存

1. 点击页面底部的 **"Generate token"**
2. **立即复制 token** (格式: `ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx`)
3. ⚠️ **重要**: 这是你唯一能看到完整 token 的机会,请妥善保存!

## 在 Infinite 中配置 Token

### 方法 1: GUI 设置 (推荐)

1. 启动 Infinite GUI
2. 点击右上角的 **⚙ 设置** 按钮
3. 在 "Token" 输入框中粘贴你的 token
4. 点击 **✅ 保存**

**说明**: GUI 会自动在调用 CLI 时传递 token,无需额外配置。

### 方法 2: 手动编辑配置文件

配置文件位置: `%APPDATA%/infinite/gui_config.json`

```json
{
  "game_path": "...",
  "mods": [...],
  "github_token": "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
}
```

### 方法 3: 环境变量 (CLI 独立使用)

如果直接使用 CLI 而不是 GUI,可以设置环境变量:

**PowerShell**:
```powershell
$env:GITHUB_TOKEN = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
infinite install --game-path "C:\path\to\game" --mod-list mods.txt
```

**CMD**:
```cmd
set GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
infinite install --game-path "C:\path\to\game" --mod-list mods.txt
```

**永久设置 (Windows)**:
1. 搜索 "环境变量"
2. 点击 "编辑系统环境变量"
3. "环境变量" → "用户变量" → "新建"
4. 变量名: `GITHUB_TOKEN`
5. 变量值: 你的 token

## 验证配置

配置成功后,GUI 右上角会显示速率限制状态:

- 🟢 **绿色** (`API: 4500/5000`): 充足,一切正常
- 🟡 **黄色** (`API: 45/5000`): 剩余较少,注意使用
- 🔴 **红色** (`API: 5/5000`): 即将耗尽,请等待重置

## 安全提示

### ✅ 安全实践

- Token 存储在本地配置文件中,仅在你的电脑上使用
- 程序只使用 token 调用 GitHub API,不会上传到其他服务器
- 使用最小权限原则 (仅 `public_repo`)

### ⚠️ 注意事项

- 不要与他人分享你的 token
- 不要将 token 提交到 Git 仓库
- 如果 token 泄露,立即在 GitHub 设置中撤销

### 撤销 Token

如果需要撤销 token:
1. 访问: https://github.com/settings/tokens
2. 找到相应的 token
3. 点击 **Delete** / **Revoke**

## 故障排除

### 错误: "GitHub API rate limit exceeded"

**原因**: 未配置 token 或 token 失效

**解决方案**:
1. 检查是否正确配置了 token
2. 验证 token 是否过期
3. 在 GitHub 设置中确认 token 仍然有效

### Token 无效

**症状**: 配置后仍然显示 60/hour 限额

**可能原因**:
1. Token 格式错误 (应该以 `ghp_` 开头)
2. Token 已被撤销
3. Token 权限不足

**解决方案**: 重新生成一个新 token

### API 限额已用完

**症状**: 显示 `API: 0/5000`

**解决方案**: 等待重置 (每小时重置一次),GUI 会显示重置倒计时

## 常见问题

### Q: Token 会过期吗?

A: 取决于你创建时选择的过期时间。建议选择 "No expiration" 以避免频繁更新。

### Q: Token 安全吗?

A: 是的,只要遵循安全实践:
- 只授予 `public_repo` 权限
- 不要分享给他人
- 定期检查 GitHub 的活跃 token 列表

### Q: 我可以使用同一个 token 在多台电脑上吗?

A: 可以,但注意速率限制是按 token (用户) 计算的,会在所有使用该 token 的设备间共享。

### Q: 未配置 token 可以使用吗?

A: 可以,但会受到更严格的限制 (60/hour vs 5000/hour)。

## 技术细节

### API 速率限制重置

- 限额每小时重置一次
- 重置时间基于第一次请求的时间
- GUI 会自动跟踪并显示剩余限额

### 缓存策略

程序会缓存以下内容以减少 API 调用:
- ✅ Mod 配置 (成功和失败状态)
- ✅ 分支列表 (会话内)
- ✅ 目录结构 (会话内)

### 请求优先级

为了最有效地使用 API 限额:
1. 优先从本地缓存加载 (`%APPDATA%/infinite/mod_cache`)
2. 如果缓存不存在,才调用 GitHub API
3. 失败的请求会被缓存,避免重复尝试

---

**相关链接**:
- [GitHub Personal Access Tokens Documentation](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token)
- [GitHub API Rate Limiting](https://docs.github.com/en/rest/overview/resources-in-the-rest-api#rate-limiting)
