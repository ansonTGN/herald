# 用户故事索引

本文档索引列出所有 Herald 系统的用户故事。

## 用户故事 ID 索引

| US-ID | 标题 | 角色 | 优先级 | 文件 |
|-------|------|------|--------|------|
| US-AR-001 | 创建 Realm | Admin Realm | P0 | [01-admin-realm](01-admin-realm-user-stories.md#故事-1创建-realm-us-ar-001) |
| US-AR-002 | 查看 Realm 列表 | Admin Realm | P0 | [01-admin-realm](01-admin-realm-user-stories.md#故事-2查看-realm-列表-us-ar-002) |
| US-AR-003 | 查看 Realm 详情 | Admin Realm | P1 | [01-admin-realm](01-admin-realm-user-stories.md#故事-3查看-realm-详情-us-ar-003) |
| US-AR-004 | Realm 创建权限控制 | Admin Realm | P0 | [01-admin-realm](01-admin-realm-user-stories.md#故事-4realm-创建权限控制-us-ar-004) |
| US-RA-001 | Realm 隔离访问 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-1realm-隔离访问-us-ra-001) |
| US-RA-002 | 角色定义管理 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-2角色定义管理-us-ra-002) |
| US-RA-003 | 权限定义管理 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-3权限定义管理-us-ra-003) |
| US-RA-004 | 为角色分配权限 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-4为角色分配权限-us-ra-004) |
| US-RA-005 | 查看角色权限 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-5查看角色权限-us-ra-005) |
| US-RA-006 | 用户角色分配 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-6用户角色分配-us-ra-006) |
| US-RA-007 | 权限策略管理 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-7权限策略管理-us-ra-007) |
| US-RA-008 | 订阅套餐管理 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-8订阅套餐管理-us-ra-008) |
| US-RA-009 | 默认角色和权限保护 | Realm Admin | P0 | [builtin_protection](builtin_protection.md#us-ra-009) |
| US-RA-010 | 查看 Dashboard 用户活跃概览 | Realm Admin | P1 | [02-realm-admin](02-realm-admin-user-stories.md#故事-10查看-dashboard-用户活跃概览-us-ra-010) |
| US-RA-011 | 查看 Dashboard 认证趋势图 | Realm Admin | P1 | [02-realm-admin](02-realm-admin-user-stories.md#故事-11查看-dashboard-认证趋势图-us-ra-011) |
| US-RA-012 | 通过 Dashboard 快捷导航跳转 | Realm Admin | P1 | [02-realm-admin](02-realm-admin-user-stories.md#故事-12通过-dashboard-快捷导航跳转-us-ra-012) |
| US-RA-013 | 配置 Realm 邮件服务 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-13配置-realm-邮件服务-us-ra-013) |
| US-RA-014 | 发送测试邮件 | Realm Admin | P1 | [02-realm-admin](02-realm-admin-user-stories.md#故事-14发送测试邮件-us-ra-014) |
| US-RA-015 | 邮件依赖的功能开关前置验证 | Realm Admin | P0 | [02-realm-admin](02-realm-admin-user-stories.md#故事-15邮件依赖的功能开关前置验证-us-ra-015) |
| US-RU-001 | 账号注册 | Regular User | P0 | [03-regular-user](03-regular-user-user-stories.md#故事-1账号注册-us-ru-001) |
| US-RU-002 | 账号登录 | Regular User | P0 | [03-regular-user](03-regular-user-user-stories.md#故事-2账号登录-us-ru-002) |
| US-RU-003 | OAuth 第三方登录 | Regular User | P0 | [03-regular-user](03-regular-user-user-stories.md#故事-3oauth-第三方登录-us-ru-003) |
| US-RU-004 | 修改个人密码 | Regular User | P1 | [03-regular-user](03-regular-user-user-stories.md#故事-4修改个人密码-us-ru-004) |
| US-RU-005 | 查看个人资料 | Regular User | P1 | [03-regular-user](03-regular-user-user-stories.md#故事-5查看个人资料-us-ru-005) |
| US-RU-006 | 修改个人昵称 | Regular User | P2 | [03-regular-user](03-regular-user-user-stories.md#故事-6修改个人昵称-us-ru-006) |
| US-RU-007 | 退出登录 | Regular User | P1 | [03-regular-user](03-regular-user-user-stories.md#故事-7退出登录-us-ru-007) |
| US-RU-008 | 访问第三方应用 | Regular User | P0 | [03-regular-user](03-regular-user-user-stories.md#故事-8访问第三方应用-us-ru-008) |
| US-RU-009 | 认证重定向流程 | All Users | P0 | [03-regular-user](03-regular-user-user-stories.md#故事-9认证重定向流程-us-ru-009) |
| US-TP-001 | OAuth 授权码登录 | Third-Party App | P0 | [04-third-party-app](04-third-party-app-user-stories.md#故事-1oauth-授权码登录-us-tp-001) |
| US-TP-002 | 验证用户登录状态 | Third-Party App | P0 | [04-third-party-app](04-third-party-app-user-stories.md#故事-2验证用户登录状态-us-tp-002) |
| US-TP-003 | 检查用户权限 | Third-Party App | P0 | [04-third-party-app](04-third-party-app-user-stories.md#故事-3检查用户权限-us-tp-003) |
| US-TP-004 | 获取用户信息 | Third-Party App | P0 | [04-third-party-app](04-third-party-app-user-stories.md#故事-4获取用户信息-us-tp-004) |
| US-TP-005 | Client App 配置管理 | Third-Party App | P0 | [04-third-party-app](04-third-party-app-user-stories.md#故事-5client-app-配置管理-us-tp-005) |
| US-TP-006 | 处理异常情况 | Third-Party App | P1 | [04-third-party-app](04-third-party-app-user-stories.md#故事-6处理异常情况-us-tp-006) |
| US-TP-007 | 会话管理 | Third-Party App | P1 | [04-third-party-app](04-third-party-app-user-stories.md#故事-7会话管理-us-tp-007) |
| US-TP-008 | 配置 Client App 跳转地址白名单 | Third-Party App | P0 | [client-app-settings](client-app-settings.md#故事-1配置-client-app-跳转地址白名单-us-tp-008) |
| US-TP-009 | 管理 Client App 图标 | Third-Party App | P0 | [client-app-settings](client-app-settings.md#故事-2管理-client-app-图标-us-tp-009) |
| US-TP-010 | 启用/禁用 Client App | Third-Party App | P0 | [client-app-settings](client-app-settings.md#故事-3启用禁用-client-app-us-tp-010) |
| US-TP-011 | 配置 Session 有效期策略 | Third-Party App | P0 | [client-app-settings](client-app-settings.md#故事-4配置-session-有效期策略-us-tp-011) |
| US-TO-001 | Realm 管理员启用/禁用 TOTP 功能 | TOTP User | P0 | [05-totp](05-totp-user-stories.md#故事-1realm-管理员启用禁用-totp-功能-us-to-001) |
| US-TO-002 | 用户启用 TOTP 二次认证 | TOTP User | P0 | [05-totp](05-totp-user-stories.md#故事-2用户启用-totp-二次认证-us-to-002) |
| US-TO-003 | 用户使用 TOTP 登录 | TOTP User | P0 | [05-totp](05-totp-user-stories.md#故事-3用户使用-totp-登录-us-to-003) |
| US-TO-004 | 用户禁用 TOTP | TOTP User | P0 | [05-totp](05-totp-user-stories.md#故事-4用户禁用-totp-us-to-004) |
| US-TO-005 | 用户重新生成 TOTP 密钥 | TOTP User | P1 | [05-totp](05-totp-user-stories.md#故事-5用户重新生成-totp-密钥-us-to-005) |
| US-TO-006 | Realm 管理员强制启用 TOTP | TOTP User | P1 | [05-totp](05-totp-user-stories.md#故事-6realm-管理员强制启用-totp-us-to-006) |
| US-TO-007 | 用户查看 TOTP 使用情况 | TOTP User | P2 | [05-totp](05-totp-user-stories.md#故事-7用户查看-totp-使用情况-us-to-007) |
| US-BI-001 | 创建订阅套餐 | Billing User | P0 | [06-billing](06-billing-user-stories.md#故事-1创建订阅套餐-us-bi-001) |
| US-BI-002 | 编辑订阅套餐 | Billing User | P0 | [06-billing](06-billing-user-stories.md#故事-2编辑订阅套餐-us-bi-002) |
| US-BI-003 | 删除订阅套餐 | Billing User | P0 | [06-billing](06-billing-user-stories.md#故事-3删除订阅套餐-us-bi-003) |
| US-BI-004 | 分配套餐到 Client App | Billing User | P0 | [06-billing](06-billing-user-stories.md#故事-4分配套餐到-client-app-us-bi-004) |
| US-BI-005 | 查看订阅列表 | Billing User | P0 | [06-billing](06-billing-user-stories.md#故事-5查看订阅列表-us-bi-005) |
| US-BI-006 | 第三方应用查询套餐状态（SDK 集成） | Billing User | P0 | [06-billing](06-billing-user-stories.md#故事-6第三方应用查询套餐状态sdk-集成-us-bi-006) |
| US-BI-007 | 查看订阅变更历史 | Billing User | P1 | [06-billing](06-billing-user-stories.md#故事-7查看订阅变更历史-us-bi-007) |
| US-BI-008 | 查看自己的订阅变更历史 | Billing User | P1 | [06-billing](06-billing-user-stories.md#故事-8查看自己的订阅变更历史-us-bi-008) |
| US-PR-001 | 创建 Product | Realm Admin | P0 | [product-management](product-management.md#故事-1创建-product-us-pr-001) |
| US-PR-002 | 编辑 Product | Realm Admin | P0 | [product-management](product-management.md#故事-2编辑-product-us-pr-002) |
| US-PR-003 | 查看 Product 列表 | Realm Admin | P0 | [product-management](product-management.md#故事-3查看-product-列表-us-pr-003) |
| US-PR-004 | 启用/禁用 Product | Realm Admin | P1 | [product-management](product-management.md#故事-4启用禁用-product-us-pr-004) |
| US-PR-005 | 删除 Product | Realm Admin | P1 | [product-management](product-management.md#故事-5删除-product-us-pr-005) |
| US-PR-006 | 在 Product 下管理 Plan | Realm Admin | P0 | [product-management](product-management.md#故事-6在-product-下管理-plan-us-pr-006) |
| US-PO-01 | 配置积分套餐 | Points Admin | P0 | [points-admin-manage](points-admin-manage.md#故事-1配置积分套餐-us-po-01) |
| US-PO-02 | 查看所有用户积分账户 | Points Admin | P1 | [points-admin-manage](points-admin-manage.md#故事-2查看所有用户积分账户-us-po-02) |
| US-PO-03 | 查看用户积分交易历史 | Points Admin | P1 | [points-admin-manage](points-admin-manage.md#故事-3查看用户积分交易历史-us-po-03) |
| US-PO-04 | 管理积分套餐配置 | Points Admin | P2 | [points-admin-manage](points-admin-manage.md#故事-4管理积分套餐配置-us-po-04) |
| US-PO-05 | 查看套餐充值引导 | Points Admin | P2 | [points-admin-manage](points-admin-manage.md#故事-5查看套餐充值引导-us-po-05) |
| US-PO-06 | 配置 Realm 默认积分策略 | Points Admin | P0 | [points-admin-manage](points-admin-manage.md#故事-6配置-realm-默认积分策略-us-po-06) |
| US-PO-07 | 查看免费用户积分统计 | Points Admin | P1 | [points-admin-manage](points-admin-manage.md#故事-7查看免费用户积分统计-us-po-07) |
| US-PU-01 | 查看我的积分余额 | Points User | P0 | [points-user-view](points-user-view.md#故事-1查看我的积分余额-us-pu-01) |
| US-PU-02 | 查看我的交易历史 | Points User | P1 | [points-user-view](points-user-view.md#故事-2查看我的交易历史-us-pu-02) |
| US-PU-03 | 筛选交易记录 | Points User | P2 | [points-user-view](points-user-view.md#故事-3筛选交易记录-us-pu-03) |
| US-PP-007 | 配置 Shopify Payment Provider | Realm Admin | P0 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-1配置-shopify-支付平台-us-pp-007) |
| US-PP-008 | 查看 Shopify Payment Provider 配置 | Realm Admin | P0 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-2查看-shopify-支付平台配置-us-pp-008) |
| US-PP-009 | 编辑 Shopify Payment Provider 配置 | Realm Admin | P1 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-3编辑-shopify-支付平台配置-us-pp-009) |
| US-PP-010 | 删除 Shopify Payment Provider 配置 | Realm Admin | P1 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-4删除-shopify-支付平台配置-us-pp-010) |
| US-PP-011 | Shopify Subscription Contract 创建和同步 | System | P0 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-5shopify-订阅合同创建与同步-us-pp-011) |
| US-PP-012 | Shopify Subscription 续费和状态同步 | System | P0 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-6shopify-订阅续费与状态同步-us-pp-012) |
| US-PP-013 | 用户认领 Shopify 订阅 | Herald User | P0 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-7用户认领-shopify-订阅-us-pp-013) |
| US-PP-014 | Webhook 处理未归属订阅 | System | P0 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-8webhook-处理未归属订阅-us-pp-014) |
| US-PP-015 | 通过 Customer Binding 自动归属 | System | P1 | [08-shopify-pay](08-shopify-pay-user-stories.md#故事-9通过-customer-binding-自动归属-us-pp-015) |
| US-WP-001 | 配置微信支付平台 | Realm Admin | P0 | [09-wechat-pay](09-wechat-pay-user-stories.md#故事-1配置微信支付平台-us-wp-001) |
| US-WP-002 | 查看微信支付平台配置 | Realm Admin | P0 | [09-wechat-pay](09-wechat-pay-user-stories.md#故事-2查看微信支付平台配置-us-wp-002) |
| US-WP-003 | 编辑微信支付平台配置 | Realm Admin | P1 | [09-wechat-pay](09-wechat-pay-user-stories.md#故事-3编辑微信支付平台配置-us-wp-003) |
| US-WP-004 | 删除微信支付平台配置 | Realm Admin | P1 | [09-wechat-pay](09-wechat-pay-user-stories.md#故事-4删除微信支付平台配置-us-wp-004) |
| US-WP-005 | 用户通过微信扫码支付 | Regular User | P0 | [09-wechat-pay](09-wechat-pay-user-stories.md#故事-5用户通过微信扫码支付-us-wp-005) |
| US-WP-006 | 微信支付 Webhook 回调处理 | System | P0 | [09-wechat-pay](09-wechat-pay-user-stories.md#故事-6微信支付-webhook-回调处理-us-wp-006) |
| US-WP-007 | 主动查询支付状态 | System | P0 | [09-wechat-pay](09-wechat-pay-user-stories.md#故事-7主动查询支付状态-us-wp-007) |
| US-WP-008 | 关闭过期支付订单 | System | P1 | [09-wechat-pay](09-wechat-pay-user-stories.md#故事-8关闭过期支付订单-us-wp-008) |
| US-PP-001 | 创建积分包 | Realm Admin | P0 | [10-points-package](10-points-package-user-stories.md#故事-1创建积分包-us-pp-001) |
| US-PP-002 | 编辑积分包 | Realm Admin | P0 | [10-points-package](10-points-package-user-stories.md#故事-2编辑积分包-us-pp-002) |
| US-PP-003 | 配置积分包的支付平台映射 | Realm Admin | P0 | [10-points-package](10-points-package-user-stories.md#故事-3配置积分包的支付平台映射-us-pp-003) |
| US-PP-004 | 查看积分包列表 | Realm Admin | P0 | [10-points-package](10-points-package-user-stories.md#故事-4查看积分包列表-us-pp-004) |
| US-PP-005 | 删除积分包 | Realm Admin | P1 | [10-points-package](10-points-package-user-stories.md#故事-5删除积分包-us-pp-005) |
| US-PU-06 | 购买积分包 | Regular User | P0 | [11-points-package-purchase](11-points-package-purchase-user-stories.md#故事-1购买积分包-us-pu-06) |
| US-PU-07 | 查看积分包购买记录 | Regular User | P1 | [11-points-package-purchase](11-points-package-purchase-user-stories.md#故事-2查看积分包购买记录-us-pu-07) |
| US-PU-08 | 积分包与订阅购买的区别 | Regular User | P1 | [11-points-package-purchase](11-points-package-purchase-user-stories.md#故事-3积分包与订阅购买的区别-us-pu-08) |
| US-PA-001 | 创建支付尝试（订阅或积分包） | System | P0 | [12-payment-attempt](12-payment-attempt-user-stories.md#故事-1创建支付尝试订阅或积分包-us-pa-001) |
| US-PA-002 | 查询支付尝试状态 | System | P0 | [12-payment-attempt](12-payment-attempt-user-stories.md#故事-2查询支付尝试状态-us-pa-002) |
| US-PA-003 | 处理支付成功后的履约 | System | P0 | [12-payment-attempt](12-payment-attempt-user-stories.md#故事-3处理支付成功后的履约-us-pa-003) |
| US-PA-004 | 关闭过期的支付尝试 | System | P1 | [12-payment-attempt](12-payment-attempt-user-stories.md#故事-4关闭过期的支付尝试-us-pa-004) |
| US-IV-001 | 创建发票 | Realm Admin | P0 | [13-invoice](13-invoice-user-stories.md#故事-1创建发票-us-iv-001) |
| US-IV-002 | 编辑发票草稿 | Realm Admin | P0 | [13-invoice](13-invoice-user-stories.md#故事-2编辑发票草稿-us-iv-002) |
| US-IV-003 | 查看发票列表 | Realm Admin | P0 | [13-invoice](13-invoice-user-stories.md#故事-3查看发票列表-us-iv-003) |
| US-IV-004 | 查看发票详情 | Realm Admin | P0 | [13-invoice](13-invoice-user-stories.md#故事-4查看发票详情-us-iv-004) |
| US-IV-005 | 开具发票 | Realm Admin | P0 | [13-invoice](13-invoice-user-stories.md#故事-5开具发票-us-iv-005) |
| US-IV-006 | 作废发票 | Realm Admin | P1 | [13-invoice](13-invoice-user-stories.md#故事-6作废发票-us-iv-006) |
| US-IV-007 | 标记发票已付 | Realm Admin | P0 | [13-invoice](13-invoice-user-stories.md#故事-7标记发票已付-us-iv-007) |
| US-IV-008 | 查看我的发票 | Regular User | P1 | [13-invoice](13-invoice-user-stories.md#故事-8查看我的发票-us-iv-008) |
| US-IV-009 | 系统标记逾期发票 | System | P1 | [13-invoice](13-invoice-user-stories.md#故事-9系统标记逾期发票-us-iv-009) |
| US-IV-010 | 配置销售方信息 | Realm Admin | P0 | [13-invoice](13-invoice-user-stories.md#故事-10配置销售方信息-us-iv-010) |
| US-IV-011 | 申请发票 | Regular User | P0 | [13-invoice](13-invoice-user-stories.md#故事-11申请发票-us-iv-011) |
| US-IV-012 | 审核并开具用户申请的发票 | Realm Admin | P0 | [13-invoice](13-invoice-user-stories.md#故事-12审核并开具用户申请的发票-us-iv-012) |
| US-AU-001 | 查看 Realm 审计日志 | Realm Admin | P0 | [14-audit](14-audit-user-stories.md#故事-1查看-realm-审计日志-us-au-001) |
| US-AU-002 | 按条件筛选审计日志 | Realm Admin | P0 | [14-audit](14-audit-user-stories.md#故事-2按条件筛选审计日志-us-au-002) |
| US-AU-003 | 查看审计日志详情 | Realm Admin | P1 | [14-audit](14-audit-user-stories.md#故事-3查看审计日志详情-us-au-003) |
| US-AU-004 | 查看 Admin Realm 审计日志 | Admin Realm | P0 | [14-audit](14-audit-user-stories.md#故事-4查看-admin-realm-审计日志-us-au-004) |
| US-AU-005 | 系统自动记录核心操作 | System | P0 | [14-audit](14-audit-user-stories.md#故事-5系统自动记录核心操作-us-au-005) |
| US-DC-001 | CLI 工具发起设备授权 | Third-Party App | P0 | [15-device-code](15-device-code-user-stories.md#故事-1cli-工具发起设备授权-us-dc-001) |
| US-DC-002 | 用户在验证页面完成授权 | Regular User | P0 | [15-device-code](15-device-code-user-stories.md#故事-2用户在验证页面完成授权-us-dc-002) |
| US-DC-003 | CLI 工具轮询获取令牌 | Third-Party App | P0 | [15-device-code](15-device-code-user-stories.md#故事-3cli-工具轮询获取令牌-us-dc-003) |
| US-DC-004 | Realm Admin 配置 Device Code Grant | Realm Admin | P1 | [15-device-code](15-device-code-user-stories.md#故事-4realm-admin-配置-device-code-grant-us-dc-004) |
| US-DC-005 | 设备验证页面 API | Third-Party App | P1 | [15-device-code](15-device-code-user-stories.md#故事-5设备验证页面-api-us-dc-005) |

**总计**: 124 个用户故事

---

## 角色分类

| 角色 | 文档 | 故事数 | 相关 PRD |
|------|------|-------|---------|
| Admin Realm 管理员 | [01-admin-realm-user-stories.md](01-admin-realm-user-stories.md) | 4 | [Realm PRD](/docs/prd/core/realm.md) |
| Realm Admin | [02-realm-admin-user-stories.md](02-realm-admin-user-stories.md), [builtin_protection.md](builtin_protection.md) | 15 | [Users PRD](/docs/prd/core/users.md), [Permissions PRD](/docs/prd/auth/permissions.md), [Client Apps PRD](/docs/prd/integration/client-app.md), [Realm Settings PRD](/docs/prd/core/realm-settings.md), [Dashboard PRD](/docs/prd/core/dashboard-redesign.md) |
| Regular User | [03-regular-user-user-stories.md](03-regular-user-user-stories.md) | 9 | [Users PRD](/docs/prd/core/users.md) |
| 第三方应用开发者 | [04-third-party-app-user-stories.md](04-third-party-app-user-stories.md), [client-app-settings.md](client-app-settings.md) | 11 | [OAuth 第三方集成 PRD](/docs/prd/auth/oauth-third-party-integration.md), [Client Apps PRD](/docs/prd/integration/client-app.md) |
| TOTP 用户 | [05-totp-user-stories.md](05-totp-user-stories.md) | 7 | [TOTP PRD](/docs/prd/auth/totp.md) |
| Billing 用户 | [06-billing-user-stories.md](06-billing-user-stories.md) | 8 | [Billing PRD](/docs/prd/billing/billing.md), [Subscription History PRD](/docs/prd/billing/subscription-history.md) |
| Product Admin | [product-management.md](product-management.md) | 6 | [Product Catalog PRD](/docs/prd/billing/product-catalog.md) |
| Points Admin | [points-admin-manage.md](points-admin-manage.md) | 7 | [Points PRD](/docs/prd/billing/points.md) |
| Points User | [points-user-view.md](points-user-view.md), [11-points-package-purchase-user-stories.md](11-points-package-purchase-user-stories.md) | 6 | [Points PRD](/docs/prd/billing/points.md), [Unified Purchase PRD](/docs/prd/billing/unified-purchase.md) |
| Payment Provider Admin | [07-payment-provider-user-stories.md](07-payment-provider-user-stories.md), [08-shopify-pay-user-stories.md](08-shopify-pay-user-stories.md) | 15 | [Billing PRD](/docs/prd/billing/billing.md), [Stripe Payment PRD](/docs/prd/billing/stripe-payment.md), [Shopify Pay PRD](/docs/prd/billing/shopify-pay.md) |
| WeChat Pay User | [09-wechat-pay-user-stories.md](09-wechat-pay-user-stories.md) | 8 | [Billing PRD](/docs/prd/billing/billing.md), [WeChat Pay PRD](/docs/prd/billing/wechat-pay.md) |
| Points Package Admin | [10-points-package-user-stories.md](10-points-package-user-stories.md) | 5 | [Unified Purchase PRD](/docs/prd/billing/unified-purchase.md) |
| Payment Attempt System | [12-payment-attempt-user-stories.md](12-payment-attempt-user-stories.md) | 4 | [Unified Purchase PRD](/docs/prd/billing/unified-purchase.md) |
| Invoice User | [13-invoice-user-stories.md](13-invoice-user-stories.md) | 9 | [Invoice PRD](/docs/prd/billing/invoice.md) |
| Audit User | [14-audit-user-stories.md](14-audit-user-stories.md) | 5 | [Audit PRD](/docs/prd/core/audit.md) |
| Device Code User | [15-device-code-user-stories.md](15-device-code-user-stories.md) | 5 | [Device Code PRD](/docs/prd/auth/device-code.md) |

## 特殊文档

| 文档 | 说明 |
|------|------|
| [_README.md](_README.md) | 用户故事编写指南 |
| [_roles.md](_roles.md) | 角色定义文档 |
| [client-app-settings.md](client-app-settings.md) | Client App 设置用户故事 |
| [builtin_protection.md](builtin_protection.md) | 内置保护功能用户故事 |

## 相关文档

- **PRD 文档索引**: [docs/prd/00-index.md](/docs/prd/00-index.md)
- **角色定义**: [_roles.md](_roles.md)
