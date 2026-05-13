# Herald Docker 构建系统

## 📋 概述

这个 Docker 构建系统用于打包 Herald (Cloud Authentication Service) 应用，包括：
- 后端 Rust 服务 (herald-app)
- 前端 React 静态文件
- PostgreSQL 和 Redis 依赖服务

## ✅ 已修复的问题

### 1. **项目结构适配**
- ❌ 原问题：Dockerfile 假设项目在根目录
- ✅ 已修复：正确处理 `backend/` 子目录结构

### 2. **Binary 名称修正**
- ❌ 原问题：使用了不存在的 `server` binary
- ✅ 已修复：使用正确的 `herald-app` binary

### 3. **构建依赖处理**
- ❌ 原问题：前端构建依赖后端但顺序错误
- ✅ 已修复：先构建后端，再用其生成前端 API 类型

### 4. **路径配置修正**
- ❌ 原问题：所有路径都指向错误位置
- ✅ 已修复：所有路径都使用正确的 `backend/` 前缀

### 5. **缓存优化**
- ✅ 新增：依赖层缓存，加速重复构建
- ✅ 新增：更好的构建失败容错处理

## 🚀 快速开始

### 前置要求
- Docker Desktop 已安装并运行
- Python 3.x 和 uv
- Node.js 和 npm (用于前端构建)
- Rust toolchain (用于后端构建)
- 网络连接正常（需访问 Docker Hub）
- 至少 4GB 可用内存

### 构建方法

#### 推荐：使用 Python 构建脚本
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

#### 直接使用 Docker
```bash
docker build -f docker/Dockerfile -t herald-app:latest .
```

## 📦 文件说明

```
docker/
├── Dockerfile              # 主要构建文件
├── docker-compose.yml      # 完整应用栈编排
├── BUILD_CHECKLIST.md     # 详细检查清单
└── README.md              # 本文件

scripts/
└── docker-build.py        # Python 构建脚本 (推荐使用)
```

## 🏗️ 构建架构

### 多阶段构建

```mermaid
graph LR
    A[Backend Builder] -->|Rust 1.83 Alpine| B[Backend Binary]
    B --> C[Frontend Builder]
    C -->|Node 20 Alpine| D[Frontend Assets]
    B --> E[Runtime Image]
    D --> E
    E -->|Alpine 3.20| F[Final Image]
```

### 阶段详解

1. **Backend Builder** (rust:1.83-alpine)
   - 编译 Rust 后端代码
   - 生成 `herald-app` binary
   - 支持依赖缓存优化

2. **Frontend Builder** (node:20-alpine)
   - 使用后端 binary 生成 OpenAPI 类型
   - 编译 React 前端
   - 生成静态资源

3. **Runtime Image** (alpine:3.20)
   - 最小化运行时镜像
   - 包含必要的运行库
   - 非 root 用户运行

## 🌐 运行应用

### 使用 Docker Compose (推荐)
```bash
cd docker
docker-compose up -d
```

这将启动：
- Herald 应用 (端口 3000)
- PostgreSQL 数据库
- Redis 缓存

访问：http://localhost:3000

### 单独运行应用
```bash
docker run -d \
  -p 3000:3000 \
  -e DATABASE_URL=postgresql://user:pass@host:5432/db \
  -e REDIS_URL=redis://host:6379 \
  herald-app:latest
```

## 🔧 配置

### 环境变量
```bash
HERALD_CONFIG=/app/config/config.toml  # 配置文件路径
DATABASE_URL=postgresql://...        # 数据库连接
REDIS_URL=redis://...                # Redis 连接
```

### 卷挂载
```bash
# 自定义配置
-v /path/to/config.toml:/app/config/config.toml

# 持久化数据
- postgres-data:/var/lib/postgresql/data
```

## 🏥 健康检查

容器包含内置健康检查：
- 间隔：30 秒
- 超时：3 秒
- 启动等待：10 秒
- 重试次数：3 次

检查状态：
```bash
docker ps
docker inspect <container_id> | grep -A 10 Health
```

## 🧪 测试构建

### 验证镜像
```bash
# 检查镜像大小
docker images | grep herald-app

# 检查镜像内容
docker run --rm herald-app:latest ls -la /app

# 测试配置
docker run --rm herald-app:latest cat /app/config/config.toml
```

### 本地测试
```bash
# 启动完整栈
cd docker && docker-compose up

# 查看日志
docker-compose logs -f herald-app

# 进入容器
docker-compose exec herald-app sh
```

## 📊 预期性能

- **镜像大小**：~100-200 MB (压缩后)
- **启动时间**：2-5 秒
- **内存使用**：~50-100 MB (运行时)
- **构建时间**：15-25 分钟 (首次)

## 🐛 故障排除

### 网络问题
```bash
# 测试 Docker Hub 连接
docker pull alpine:3.20

# 配置镜像加速器 (国内)
# Docker Desktop -> Settings -> Docker Engine
# 添加: "registry-mirrors": ["https://mirror.ccs.tencentyun.com"]
```

### 构建失败
```bash
# 清理缓存重新构建
docker build --no-cache -f docker/Dockerfile -t herald-app:latest .

# 查看详细日志
docker build --progress=plain -f docker/Dockerfile -t herald-app:latest .
```

### 运行时问题
```bash
# 查看应用日志
docker logs <container_id>

# 进入容器调试
docker exec -it <container_id> sh

# 检查配置
docker exec <container_id> cat /app/config/config.toml
```

## 🔄 更新与维护

### 重新构建
```bash
# 拉取最新代码
git pull

# 重新构建镜像
uv run scripts/docker-build.py

# 或使用自定义标签
uv run scripts/docker-build.py --tag v2.0.0
```

### 清理旧资源
```bash
# 停止所有容器
docker-compose down

# 删除旧镜像
docker rmi herald-app:latest

# 清理未使用资源
docker system prune -a
```

## 📝 最佳实践

1. **开发环境**：使用 docker-compose 进行本地开发
2. **生产环境**：使用环境变量覆盖配置
3. **持续集成**：使用 `--build-arg` 支持构建参数
4. **安全扫描**：定期使用 `docker scan` 检查漏洞
5. **版本管理**：使用 git tag 作为镜像标签

## 🆘 获取帮助

如遇到问题，请检查：
1. [BUILD_CHECKLIST.md](BUILD_CHECKLIST.md) - 详细检查清单
2. Docker 日志：`docker logs <container>`
3. 应用配置：`/app/config/config.toml`
4. 健康状态：`docker inspect <container>`

## 📄 许可证

Apache-2.0 - 与主项目保持一致
