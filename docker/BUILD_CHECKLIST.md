# Herald Docker 构建检查清单

## 修复总结

已修复的 Dockerfile 问题：

1. **✅ 项目结构修复**
   - 原：假设项目在根目录
   - 新：正确处理 `backend/` 子目录结构

2. **✅ Binary 名称修复**
   - 原：`server`
   - 新：`herald-app`

3. **✅ 构建顺序优化**
   - 原：先构建前端（会失败，因为依赖后端）
   - 新：先构建后端，再使用后端 binary 生成前端 API 类型

4. **✅ 路径修复**
   - 所有后端相关路径都已调整为正确的 `backend/` 前缀

5. **✅ 依赖修复**
   - 添加了 `openssl-dev` 用于某些 Rust crates
   - 处理了前端 `prebuild` 钩子对后端的依赖

## 构建前检查

### 1. 网络连接
确保可以访问 Docker Hub：
```bash
docker pull alpine:3.20
```

### 2. Docker 状态
确保 Docker 正在运行：
```bash
docker version
docker ps
```

### 3. 文件完整性
确保以下文件存在：
```bash
# 检查关键文件
ls docker/Dockerfile
ls backend/Cargo.toml
ls backend/api/config/config.toml
ls frontend/package.json
```

## 构建方法

### 方法 1：使用 Python 脚本（推荐）
```bash
# 基本构建
uv run scripts/docker-build.py

# 自定义标签
uv run scripts/docker-build.py --tag v1.0.0

# 构建并推送到仓库
uv run scripts/docker-build.py --push --registry registry.example.com/team

# 详细输出
uv run scripts/docker-build.py --verbose
```

### 方法 2：直接使用 Docker 命令
```bash
docker build -f docker/Dockerfile -t herald-app:latest .
```

### 方法 3：使用 Docker Compose
```bash
cd docker
docker-compose build
```

## 构建验证

构建成功后，验证镜像：

```bash
# 检查镜像
docker images | grep herald-app

# 查看镜像详情
docker inspect herald-app:latest

# 测试运行（不启动依赖服务）
docker run --rm herald-app:latest ls -la /app
```

## 预期构建时间

- 后端构建：10-20 分钟（取决于机器性能）
- 前端构建：2-5 分钟
- 总计：15-25 分钟

## 常见问题排查

### 问题 1：网络连接失败
**症状**：`failed to authorize: failed to fetch anonymous token`

**解决**：
- 检查网络连接
- 配置 Docker 镜像加速器（如果在国内）
- 重启 Docker Desktop

### 问题 2：前端 API 生成失败
**症状**：`API generation failed (will use existing types if available)`

**解决**：
- 这是警告，脚本会使用现有类型继续构建
- 如果没有现有类型，需要手动运行一次后端生成 API：
  ```bash
  cd frontend
  npm run generate-api
  ```

### 问题 3：Python 脚本执行失败
**症状**：`ModuleNotFoundError: No module named 'lib'`

**解决**：
- 确保在项目根目录运行脚本
- 使用 `uv run scripts/docker-build.py` 而不是直接 `python scripts/docker-build.py`
- 检查 `scripts/lib/` 目录是否存在

### 问题 3：内存不足
**症状**：`compilation failed` 或 `out of memory`

**解决**：
- 增加 Docker 内存限制（Docker Desktop -> Settings -> Resources）
- 或者在主机上构建后复制到容器

### 问题 4：权限问题
**症状**：`permission denied`

**解决**：
```bash
# Windows：以管理员身份运行 PowerShell
# Linux/Mac：使用 sudo
sudo docker build -f docker/Dockerfile -t herald-app:latest .
```

## 运行完整应用

使用 Docker Compose 启动完整应用栈：

```bash
cd docker
docker-compose up -d
```

这将启动：
- Herald 应用（端口 3000）
- PostgreSQL 数据库
- Redis 缓存

访问：http://localhost:3000

## 清理

```bash
# 停止并删除容器
docker-compose down

# 删除镜像
docker rmi herald-app:latest

# 清理所有未使用的资源
docker system prune -a
```
