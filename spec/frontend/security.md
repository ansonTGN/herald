# 前端安全规范

> **版本**: 1.0  
> **状态**: 生效  
> **最后更新**: 2026-04-10

## 1. 认证与授权

### 1.1 Token 管理

- **存储位置**: Token 必须存储在 httpOnly cookie 中，不得存储在 localStorage 或 sessionStorage
- **传输方式**: 所有 API 调用必须通过 Cookie 自动附加认证信息
- **Token 刷新**: 使用 refresh token 机制，自动处理过期

### 1.2 路由保护

- **认证检查**: 路由级必须验证用户认证状态
- **权限验证**: 页面级必须验证用户权限（realm 角色）
- **重定向**: 未认证用户重定向到登录页

## 2. 数据保护

### 2.1 敏感数据处理

- **不在 URL 中**: 敏感数据（token、密码、密钥）不得出现在 URL 参数或 hash 中
- **不存储敏感信息**: 密码、TOTP 密钥等不得存储在前端（除临时会话）
- **日志脱敏**: 控制台日志和错误报告必须脱敏敏感信息

### 2.2 表单验证

- **客户端验证**: 第一道防线，提供即时反馈
- **服务端验证**: 必不可少，客户端验证可被绕过
- **输入清理**: 防止 XSS，所有用户输入必须清理

### 2.3 API 响应处理

- **错误消息**: 不泄露系统信息（路径、表结构等）
- **敏感数据过滤**: API 响应不得包含不必要的数据
- **错误日志**: 安全相关错误必须记录和监控

## 3. Content Security Policy (CSP)

### 3.1 CSP 头配置

```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'unsafe-inline' 'unsafe-eval';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: https:;
  font-src 'self' data:;
  connect-src 'self' https://api.example.com;
  frame-ancestors 'none';
```

### 3.2 内联脚本限制

- **避免内联脚本**: 尽量使用外部脚本文件
- **unsafe-inline**: 仅用于必要的动态脚本（如 analytics）
- **nonce/hash**: 生产环境使用 nonce 或 hash 替代 unsafe-inline

## 4. HTTPS 与传输安全

### 4.1 强制 HTTPS

- **生产环境**: 仅允许 HTTPS 连接
- **开发环境**: 可使用 HTTP，但警告差异
- **混合内容**: 禁止在 HTTPS 页面加载 HTTP 资源

### 4.2 API 通信

- **加密传输**: 所有 API 调用必须使用 HTTPS
- **证书验证**: 不禁用 SSL 证书验证
- **HSTS**: 启用 HTTP Strict Transport Security

## 5. 第三方依赖安全

### 5.1 依赖管理

- **定期更新**: 保持依赖包最新，修复已知漏洞
- **审计**: 使用 `npm audit` 定期检查漏洞
- **许可证**: 遵守所有依赖的开源许可证

### 5.2 外部脚本

- **最小化**: 仅加载必要的外部脚本
- **SRI**: 使用 Subresource Integrity 验证外部脚本
- **CSP**: 通过 CSP 限制外部脚本来源

## 6. 前端特定安全

### 6.1 DOM 操作

- **避免 innerHTML**: 优先使用 textContent 或 createElement
- **XSS 防护**: 用户输入必须转义后再插入 DOM
- **危险 API**: 避免使用 dangerouslySetInnerHTML

### 6.2 状态管理

- **不存储敏感数据**: Redux/Zustand 状态不包含敏感信息
- **内存清理**: 组件卸载时清理敏感数据
- **开发工具**: 生产环境禁用 React DevTools

## 7. 安全测试

### 7.1 自动化检查

- **ESLint**: 使用安全插件（如 eslint-plugin-security）
- **依赖扫描**: CI/CD 中运行 npm audit
- **SRI 检查**: 验证外部脚本完整性

### 7.2 手动测试

- **渗透测试**: 定期进行安全渗透测试
- **代码审查**: 安全相关代码必须经过审查
- **漏洞响应**: 建立漏洞响应流程

## 8. 安全监控

### 8.1 错误追踪

- **不记录敏感信息**: 错误日志脱敏
- **安全事件**: 记录安全相关事件（认证失败、授权失败）
- **实时告警**: 严重安全事件实时告警

### 8.2 性能监控

- **异常检测**: 监控异常 API 调用模式
- **流量分析**: 检测异常流量模式
- **用户行为**: 监控异常用户行为

---

## 参考资源

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [MDN Web Security](https://developer.mozilla.org/en-US/docs/Web/Security)
- [CSP Level 3](https://www.w3.org/TR/CSP3/)

---

**维护**: 前端团队  
**审查**: 安全团队  
**生效日期**: 2026-04-10
