export interface HomeTexts {
  badge: string;
  heroTitle: string;
  heroDesc: string;
  getStarted: string;
  liveDemo: string;
  viewFeatures: string;
  terminal: {
    label: string;
    lines: {
      prefix?: string;
      text: string;
      status?: "ok" | "info" | "command";
    }[];
  };
  featureSectionTitle: string;
  featureSectionDesc: string;
  features: {
    title: string;
    desc: string;
    bullets: string[];
  }[];
  stepsSectionTitle: string;
  stepsSectionDesc: string;
  steps: {
    num: string;
    title: string;
    desc: string;
  }[];
  compareSectionTitle: string;
  compareSectionDesc: string;
  compareHeaders: {
    herald: string;
    auth0: string;
    supabase: string;
    keycloak: string;
  };
  compareRows: {
    label: string;
    herald: string;
    auth0: string;
    supabase: string;
    keycloak: string;
  }[];
  faqSectionTitle: string;
  faqSectionDesc: string;
  faq: {
    question: string;
    answer: string;
  }[];
  ctaTitle: string;
  ctaDesc: string;
  starGithub: string;
  readDocs: string;
  footer: {
    copyright: string;
    privacy: string;
    terms: string;
  };
}

export const en: HomeTexts = {
  badge: "Open Source · Self-Hosted",
  heroTitle:
    "Herald ships a complete multi-tenant platform — auth, billing, payments, user management — so small teams skip infrastructure entirely.",
  heroDesc:
    "AI-assisted customization means you tailor it to your needs without touching boilerplate.",
  getStarted: "Get Started",
  liveDemo: "Live Demo",
  viewFeatures: "View Features",
  terminal: {
    label: "terminal",
    lines: [
      {
        prefix: "$",
        text: "git clone https://github.com/timzaak/herald.git",
        status: "command",
      },
      { prefix: "$", text: "cd herald", status: "command" },
      { prefix: "$", text: "uv run scripts/dev-start.py", status: "command" },
      { text: "→ Starting PostgreSQL + Redis ...", status: "info" },
      { text: "✓ Database migrated", status: "ok" },
      { text: "✓ Multi-tenant auth  (RBAC, OAuth, TOTP)", status: "ok" },
      { text: "✓ Subscription billing (Stripe, WeChat Pay)", status: "ok" },
      { text: "✓ Admin console @ http://localhost:3000", status: "ok" },
      {
        text: "→ Your auth & billing infrastructure is ready. Focus on your product.",
        status: "info",
      },
    ],
  },
  featureSectionTitle: "Everything except your core product",
  featureSectionDesc:
    "Auth, billing, payments, user management — the stuff every SaaS needs but nobody wants to build. It's all here, AI-customizable, ready to deploy.",
  features: [
    {
      title: "Multi-Tenant Auth",
      desc: "Organize users into isolated Realms with full data separation. Each Realm gets its own users, roles, OAuth providers, and Client Apps.",
      bullets: [
        "Realm-based tenant isolation",
        "OAuth 2.0 provider (Google, GitHub, WeChat)",
        "TOTP two-factor authentication",
      ],
    },
    {
      title: "RBAC & Client Apps",
      desc: "Fine-grained role-based access control per Realm. Register Client Apps with OAuth 2.0 credentials and control which apps access which resources.",
      bullets: [
        "Role-based permissions per Realm",
        "Client App registration & secrets",
        "Third-party API integration",
      ],
    },
    {
      title: "Billing & Payments",
      desc: "Create subscription plans, map them to payment providers, and assign plans to Client Apps. Includes points/credits and invoice management.",
      bullets: [
        "Subscription plans & pricing tiers",
        "Stripe & WeChat Pay integration",
        "Points & credits system",
      ],
    },
    {
      title: "Consent & Compliance",
      desc: "Version Terms and Privacy Policies per Realm, hosted as Herald content or your own external pages.",
      bullets: [
        "Independent hosting mode per agreement",
        "Version-bound consent and re-consent",
        "Auditable publishing and account deletion",
      ],
    },
  ],
  stepsSectionTitle: "From zero to production in three steps",
  stepsSectionDesc:
    "Deploy the platform. Let AI customize it. Ship your product.",
  steps: [
    {
      num: "01",
      title: "Deploy with Docker",
      desc: "Clone the repo, point your domain, and run dev-start.py. PostgreSQL, Redis, Caddy (with auto-TLS), and the Herald app start together on one machine.",
    },
    {
      num: "02",
      title: "Customize with AI",
      desc: "Create Realms, set up OAuth providers (Google, GitHub, WeChat), configure RBAC roles. Use AI-assisted tools to tailor the platform without hand-writing infrastructure code.",
    },
    {
      num: "03",
      title: "Connect Your Apps",
      desc: "Your applications authenticate users through Herald's OAuth 2.0 endpoints. Users sign in with email/password or social logins. Herald handles sessions, tokens, and user management.",
    },
  ],
  compareSectionTitle: "Why small teams choose Herald",
  compareSectionDesc:
    "Auth, billing, and payments in one self-hosted system. AI helps you customize. No stitching services together.",
  compareHeaders: {
    herald: "Herald",
    auth0: "Auth0",
    supabase: "Supabase",
    keycloak: "Keycloak",
  },
  compareRows: [
    {
      label: "Multi-tenant auth",
      herald: "Included",
      auth0: "Enterprise only",
      supabase: "Manual setup",
      keycloak: "Included",
    },
    {
      label: "Subscription billing",
      herald: "Built-in",
      auth0: "—",
      supabase: "—",
      keycloak: "—",
    },
    {
      label: "Points & credits",
      herald: "Built-in",
      auth0: "—",
      supabase: "—",
      keycloak: "—",
    },
    {
      label: "Self-hosted",
      herald: "Yes",
      auth0: "Cloud only",
      supabase: "Yes",
      keycloak: "Yes",
    },
    {
      label: "Open source",
      herald: "Apache-2.0",
      auth0: "No",
      supabase: "Partial",
      keycloak: "Apache-2.0",
    },
  ],
  faqSectionTitle: "Frequently asked questions",
  faqSectionDesc: "Everything you need to know about Herald.",
  faq: [
    {
      question: "What is Herald?",
      answer:
        "Herald is an open-source, self-hosted multi-tenant auth, billing, and payments platform. It ships with Realm-based tenant isolation, OAuth 2.0 providers, TOTP two-factor auth, RBAC, Client App management, subscription billing, and a points/credits system.",
    },
    {
      question: "How is Herald different from Auth0 or Keycloak?",
      answer:
        "Herald combines authentication and billing in one self-hosted system with AI-assisted customization. Auth0 is cloud-only and charges per user. Keycloak is self-hosted but has no billing. Herald gives you multi-tenant auth plus subscription management, points/credits, and payment integration.",
    },
    {
      question: "What does multi-tenant mean in Herald?",
      answer:
        "Multi-tenant means Herald organizes your users and data into isolated Realms. Each Realm is a separate tenant with its own users, OAuth providers, Client Apps, and billing plans. Data between Realms is fully isolated.",
    },
    {
      question: "How do I deploy Herald?",
      answer:
        "Herald deploys with Docker. You need a Linux server (Ubuntu 22.04+, 2GB RAM), Docker Engine 24+, and a domain. Four containers run together: the Herald app, PostgreSQL, Redis, and Caddy.",
    },
    {
      question: "What payment providers does Herald support?",
      answer:
        "Herald supports Stripe and WeChat Pay for subscription payments. You can create subscription plans with different pricing tiers, map plans to specific payment providers, and assign plans to Client Apps.",
    },
    {
      question: "What tech stack does Herald use?",
      answer:
        "Herald uses Rust (Axum framework) for the backend API and React with TypeScript for the frontend. Data is stored in PostgreSQL with SeaORM, and Redis handles sessions and caching.",
    },
    {
      question: "Is Herald free and open source?",
      answer:
        "Yes. Herald is released under the Apache-2.0 license. You can use, modify, and distribute it freely, including for commercial projects. There are no usage limits and no per-user fees.",
    },
  ],
  ctaTitle: "Stop building infrastructure. Start shipping product.",
  ctaDesc:
    "Herald gives you auth, billing, payments, and user management out of the box. AI handles customization. You focus on what makes your software unique.",
  starGithub: "Star on GitHub",
  readDocs: "Read Docs",
  footer: {
    copyright: "Herald · Apache 2.0",
    privacy: "Privacy",
    terms: "Terms",
  },
};

export const zh: HomeTexts = {
  badge: "开源 · 自托管",
  heroTitle:
    "Herald 提供完整的 multi-tenant 平台 —— 认证、计费、支付、用户管理一应俱全，让小团队跳过基础设施搭建。",
  heroDesc: "AI 辅助定制意味着你可以按需调整，而无需编写样板代码。",
  getStarted: "快速开始",
  liveDemo: "在线演示",
  viewFeatures: "查看功能",
  terminal: {
    label: "terminal",
    lines: [
      {
        prefix: "$",
        text: "git clone https://github.com/timzaak/herald.git",
        status: "command",
      },
      { prefix: "$", text: "cd herald", status: "command" },
      { prefix: "$", text: "uv run scripts/dev-start.py", status: "command" },
      { text: "→ 正在启动 PostgreSQL + Redis ...", status: "info" },
      { text: "✓ 数据库迁移完成", status: "ok" },
      { text: "✓ Multi-tenant 认证 (RBAC, OAuth, TOTP)", status: "ok" },
      { text: "✓ 订阅计费 (Stripe, 微信支付)", status: "ok" },
      { text: "✓ 管理后台 @ http://localhost:3000", status: "ok" },
      { text: "→ 认证与计费基础设施已就绪，专注于你的产品。", status: "info" },
    ],
  },
  featureSectionTitle: "除核心产品外的一切",
  featureSectionDesc:
    "认证、计费、支付、用户管理 —— 每个 SaaS 都需要但没人愿意从零搭建的东西。Herald 已内置，支持 AI 定制，随时部署。",
  features: [
    {
      title: "多租户认证",
      desc: "将用户组织到相互隔离的 Realm 中，实现完整的数据隔离。每个 Realm 拥有独立的用户、角色、OAuth 提供商和客户端应用。",
      bullets: [
        "基于 Realm 的租户隔离",
        "OAuth 2.0 提供商（Google、GitHub、微信）",
        "TOTP 双因素认证",
      ],
    },
    {
      title: "RBAC 与客户端应用",
      desc: "每个 Realm 内细粒度的基于角色的访问控制。注册带有 OAuth 2.0 凭证的客户端应用，并控制哪些应用可以访问哪些资源。",
      bullets: [
        "每个 Realm 的基于角色权限",
        "客户端应用注册与密钥管理",
        "第三方 API 集成",
      ],
    },
    {
      title: "计费与支付",
      desc: "创建订阅计划，映射到支付提供商，并分配给客户端应用。内置积分/信用点与发票管理。",
      bullets: [
        "订阅计划与定价层级",
        "Stripe 与微信支付集成",
        "积分与信用点系统",
      ],
    },
    {
      title: "知情同意与合规",
      desc: "按 Realm 对用户协议和隐私政策独立版本化，可由 Herald 承载全文，也可链接到你自己的外部页面。",
      bullets: [
        "每份协议独立选择承载模式",
        "绑定版本的同意与重新同意",
        "可审计的发布和账户注销",
      ],
    },
  ],
  stepsSectionTitle: "三步从零到生产",
  stepsSectionDesc: "部署平台，让 AI 定制，发布产品。",
  steps: [
    {
      num: "01",
      title: "Docker 部署",
      desc: "克隆仓库，指向你的域名，运行 dev-start.py。PostgreSQL、Redis、Caddy（自动 TLS）与 Herald 应用在同一台机器上启动。",
    },
    {
      num: "02",
      title: "AI 定制",
      desc: "创建 Realm，设置 OAuth 提供商（Google、GitHub、微信），配置 RBAC 角色。借助 AI 辅助工具定制平台，无需手写基础设施代码。",
    },
    {
      num: "03",
      title: "接入你的应用",
      desc: "你的应用通过 Herald 的 OAuth 2.0 端点认证用户。用户可使用邮箱/密码或社交登录。Herald 负责会话、令牌与用户管理。",
    },
  ],
  compareSectionTitle: "小团队为何选择 Herald",
  compareSectionDesc:
    "认证、计费、支付一体化自托管系统。AI 辅助定制，无需拼接多种服务。",
  compareHeaders: {
    herald: "Herald",
    auth0: "Auth0",
    supabase: "Supabase",
    keycloak: "Keycloak",
  },
  compareRows: [
    {
      label: "多租户认证",
      herald: "内置",
      auth0: "企业版",
      supabase: "需手动配置",
      keycloak: "内置",
    },
    {
      label: "订阅计费",
      herald: "内置",
      auth0: "—",
      supabase: "—",
      keycloak: "—",
    },
    {
      label: "积分与信用点",
      herald: "内置",
      auth0: "—",
      supabase: "—",
      keycloak: "—",
    },
    {
      label: "自托管",
      herald: "支持",
      auth0: "仅云",
      supabase: "支持",
      keycloak: "支持",
    },
    {
      label: "开源",
      herald: "Apache-2.0",
      auth0: "否",
      supabase: "部分",
      keycloak: "Apache-2.0",
    },
  ],
  faqSectionTitle: "常见问题",
  faqSectionDesc: "关于 Herald 你需要知道的一切。",
  faq: [
    {
      question: "Herald 是什么？",
      answer:
        "Herald 是一个开源、自托管的多租户认证、计费和支付平台。它内置 Realm 租户隔离、OAuth 2.0 提供商、TOTP 双因素认证、RBAC、客户端应用管理、订阅计费以及积分/信用点系统。",
    },
    {
      question: "Herald 与 Auth0 或 Keycloak 有何不同？",
      answer:
        "Herald 将认证与计费整合在一个自托管系统中，并支持 AI 辅助定制。Auth0 仅提供云服务且按用户收费，Keycloak 是自托管的但不支持计费。Herald 在提供多租户认证的同时，还支持订阅管理、积分/信用点和支付集成。",
    },
    {
      question: "Herald 中的多租户是什么意思？",
      answer:
        "多租户意味着 Herald 将用户和数据组织到相互隔离的 Realm 中。每个 Realm 是独立的租户，拥有自己的用户、OAuth 提供商、客户端应用和计费计划。Realm 之间的数据完全隔离。",
    },
    {
      question: "如何部署 Herald？",
      answer:
        "Herald 通过 Docker 部署。你需要一台 Linux 服务器（Ubuntu 22.04+，2GB 内存）、Docker Engine 24+ 和一个域名。四个容器一起运行：Herald 应用、PostgreSQL、Redis 和 Caddy。",
    },
    {
      question: "Herald 支持哪些支付提供商？",
      answer:
        "Herald 支持 Stripe 和微信支付用于订阅付款。你可以创建不同定价层级的订阅计划，将其映射到特定支付提供商，并分配给客户端应用。",
    },
    {
      question: "Herald 使用什么技术栈？",
      answer:
        "Herald 后端使用 Rust（Axum 框架）提供 API，前端使用 React + TypeScript。数据存储在 PostgreSQL 中，使用 SeaORM；Redis 负责会话与缓存。",
    },
    {
      question: "Herald 是否免费开源？",
      answer:
        "是的。Herald 采用 Apache-2.0 许可证发布。你可以自由使用、修改和分发，包括商业项目。没有使用限制，也不按用户收费。",
    },
  ],
  ctaTitle: "停止搭建基础设施，开始发布产品。",
  ctaDesc:
    "Herald 开箱即用地提供认证、计费、支付和用户管理。AI 负责定制，你专注于让软件与众不同的地方。",
  starGithub: "Star on GitHub",
  readDocs: "阅读文档",
  footer: {
    copyright: "Herald · Apache 2.0",
    privacy: "隐私政策",
    terms: "服务条款",
  },
};
