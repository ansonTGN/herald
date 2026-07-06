export interface HomeTexts {
  badge: string;
  heroTitleBefore: string;
  heroTitleEm: string;
  heroDesc1: string;
  heroDesc2: string;
  starGithub: string;
  readDocs: string;
  featureTitle: string;
  featureDesc: string;
  features: { title: string; desc: string }[];
  howTitle: string;
  howDesc: string;
  steps: { title: string; desc: string }[];
  whyTitle: string;
  whyDesc: string;
  tableRows: { label: string; herald: string; typical: string }[];
  ctaTitle: string;
  ctaDesc: string;
  getStarted: string;
}

export const en: HomeTexts = {
  badge: "Open source · Apache 2.0",
  heroTitleBefore: "An SaaS foundation,",
  heroTitleEm: "ready to ship.",
  heroDesc1: "Multi-tenant accounts, Stripe / Creem payments, and a built-in credits system.",
  heroDesc2: "Rust backend + React frontend, single process, Docker in production.",
  starGithub: "Star on GitHub",
  readDocs: "Read Docs",
  featureTitle: "Everything you need to launch",
  featureDesc: "Accounts, auth, billing, and credits — wired together out of the box.",
  features: [
    { title: "Multi-tenant", desc: "Realm-based isolation per tenant" },
    { title: "Auth & OAuth", desc: "Email, Google, GitHub, Apple, WeChat, TOTP" },
    { title: "Stripe & Creem", desc: "Subscriptions, invoices, webhooks" },
    { title: "Credits wallet", desc: "Transactions, scheduled grants, expiry" },
    { title: "Admin console", desc: "Users, roles, permissions built in" },
    { title: "Rust + React", desc: "Single process, Docker deployment" },
  ],
  howTitle: "How it works",
  howDesc: "From empty box to running SaaS in three steps.",
  steps: [
    {
      title: "Start services",
      desc: "Bring up PostgreSQL and Redis containers, then run the backend and frontend.",
    },
    {
      title: "Create a realm",
      desc: "The admin user you nominate gets super-admin rights to invite users and configure permissions.",
    },
    {
      title: "Wire payments",
      desc: "Plug in your Stripe or Creem keys, configure entitlements, and the credits system does the rest.",
    },
  ],
  whyTitle: "Why Herald",
  whyDesc: "Compared to assembling an SaaS stack yourself.",
  tableRows: [
    { label: "Account system", herald: "Multi-tenant, ready", typical: "Build from scratch" },
    { label: "Payments", herald: "Stripe + Creem integrated", typical: "Integrate each provider" },
    { label: "Credits", herald: "Wallet, grants, expiry", typical: "Custom ledger" },
    { label: "Time to launch", herald: "Days", typical: "Weeks to months" },
  ],
  ctaTitle: "Open source, Apache 2.0.",
  ctaDesc: "Run it locally in minutes.",
  getStarted: "Get Started",
};

export const zh: HomeTexts = {
  badge: "开源 · Apache 2.0",
  heroTitleBefore: "开箱即用的",
  heroTitleEm: "SaaS 底座。",
  heroDesc1: "多租户账户体系、对接 Stripe / Creem 支付、自带积分系统。",
  heroDesc2: "Rust 后端 + React 前端，单体部署，Docker 上线。",
  starGithub: "Star on GitHub",
  readDocs: "阅读文档",
  featureTitle: "开箱即用的 SaaS 能力",
  featureDesc: "账户、认证、计费、积分，全部接通。",
  features: [
    { title: "多租户", desc: "基于 Realm 的租户隔离" },
    { title: "认证与 OAuth", desc: "邮箱、Google、GitHub、Apple、微信、TOTP" },
    { title: "Stripe 与 Creem", desc: "订阅、发票、Webhook" },
    { title: "积分钱包", desc: "交易、定时发放、过期" },
    { title: "管理后台", desc: "内置用户、角色、权限" },
    { title: "Rust + React", desc: "单体进程，Docker 部署" },
  ],
  howTitle: "工作原理",
  howDesc: "三步从空机器到运行中的 SaaS。",
  steps: [
    {
      title: "启动服务",
      desc: "启动 PostgreSQL 和 Redis 容器，然后跑起后端和前端。",
    },
    {
      title: "创建 Realm",
      desc: "你在创建 Realm 时指定的管理员用户拥有超级权限，可以邀请用户、配置权限。",
    },
    {
      title: "接入支付",
      desc: "填入 Stripe 或 Creem 密钥，配置 Entitlement，积分系统会自动处理剩下的。",
    },
  ],
  whyTitle: "为什么选择 Herald",
  whyDesc: "与自行搭建 SaaS 技术栈对比。",
  tableRows: [
    { label: "账户体系", herald: "多租户，开箱即用", typical: "从零自建" },
    { label: "支付", herald: "Stripe + Creem 已集成", typical: "逐个对接" },
    { label: "积分", herald: "钱包、发放、过期", typical: "自行实现账本" },
    { label: "上线周期", herald: "数天", typical: "数周到数月" },
  ],
  ctaTitle: "开源，Apache 2.0 协议。",
  ctaDesc: "几分钟本地跑起来。",
  getStarted: "快速开始",
};
