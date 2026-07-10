import { HomeLayout } from "fumadocs-ui/layouts/home";
import { baseOptions, SiteFooter } from "@/lib/layout.shared";

interface PrivacyTexts {
  title: string;
  updated: string;
  sections: { heading: string; body: string }[];
  footer: {
    copyright: string;
    privacy: string;
    terms: string;
  };
}

const en: PrivacyTexts = {
  title: "Privacy Policy",
  updated: "Last updated: July 2026",
  sections: [
    {
      heading: "Overview",
      body: "Herald is an open-source, self-hosted software project. We do not operate a cloud service, and we do not collect, store, or process personal data on your behalf through this website or our software.",
    },
    {
      heading: "Website",
      body: "This landing page uses Google Analytics to collect anonymous usage data such as page views, referral sources, and general geographic regions. We use this data solely to understand how visitors find and interact with the site. Google Analytics uses cookies to distinguish users. You can opt out by installing the Google Analytics Opt-out Browser Add-on. We do not collect personally identifiable information through this website.",
    },
    {
      heading: "Self-Hosted Software",
      body: "When you deploy Herald on your own infrastructure, you are solely responsible for the data you collect and process. Herald does not phone home, send telemetry, or transmit any data to external servers. All data remains within your own deployment.",
    },
    {
      heading: "GitHub",
      body: "Our source code is hosted on GitHub. If you interact with our repository (issues, discussions, pull requests), GitHub's own Privacy Statement applies to those interactions.",
    },
    {
      heading: "Third-Party Services",
      body: "Herald integrates with services like Stripe for payments and supports various OAuth providers for authentication. When you configure these integrations in your self-hosted instance, the respective privacy policies of those services apply. Herald itself does not act as an intermediary for data shared with these providers.",
    },
    {
      heading: "Changes",
      body: "We may update this policy from time to time. Any changes will be reflected on this page with an updated revision date. The latest version is always available in our GitHub repository.",
    },
    {
      heading: "Contact",
      body: "If you have questions about this policy, please open an issue on our GitHub issue tracker.",
    },
  ],
  footer: {
    copyright: "Herald · Apache 2.0",
    privacy: "Privacy",
    terms: "Terms",
  },
};

const zh: PrivacyTexts = {
  title: "隐私政策",
  updated: "最后更新：2026 年 7 月",
  sections: [
    {
      heading: "概述",
      body: "Herald 是一个开源、自托管的软件项目。我们不运营云服务，也不会通过本网站或软件代表你收集、存储或处理个人数据。",
    },
    {
      heading: "网站",
      body: "本落地页使用 Google Analytics 收集匿名使用数据，例如页面浏览量、来源和大概地理位置。我们仅使用这些数据了解访客如何找到并与网站互动。Google Analytics 使用 Cookie 区分用户，你可以安装 Google Analytics 退出浏览器插件来停用。我们不会通过本网站收集可识别个人身份的信息。",
    },
    {
      heading: "自托管软件",
      body: "在你自己的基础设施上部署 Herald 时，你需自行负责所收集和处理的数据。Herald 不会回连、发送遥测数据或向外部服务器传输任何数据，所有数据都保留在你自己的部署环境中。",
    },
    {
      heading: "GitHub",
      body: "我们的源代码托管在 GitHub 上。如果你与我们的仓库互动（issue、讨论、pull request），适用 GitHub 自身的隐私声明。",
    },
    {
      heading: "第三方服务",
      body: "Herald 集成了 Stripe 等支付服务，并支持多种 OAuth 提供商进行认证。在你自托管实例中配置这些集成时，适用各服务各自的隐私政策。Herald 本身不会作为你与这些提供商共享数据的中介。",
    },
    {
      heading: "变更",
      body: "我们可能会不时更新本政策。任何变更都会在本页面显示更新后的修订日期。最新版本始终可在我们的 GitHub 仓库中获取。",
    },
    {
      heading: "联系我们",
      body: "如果你对本政策有疑问，请在 GitHub issue 跟踪器中提交 issue。",
    },
  ],
  footer: {
    copyright: "Herald · Apache 2.0",
    privacy: "隐私政策",
    terms: "服务条款",
  },
};

const textsMap: Record<string, PrivacyTexts> = { en, zh };

export function PrivacyPage({ lang }: { lang: string }) {
  const t = textsMap[lang] ?? en;
  return (
    <HomeLayout {...baseOptions()}>
      <div className="relative z-10 py-20 px-4 min-h-[60vh]">
        <div className="max-w-3xl mx-auto">
          <h1 className="text-4xl md:text-5xl font-serif font-bold text-stone-900 dark:text-stone-100 mb-2 tracking-tight">
            {t.title}
          </h1>
          <p className="text-stone-500 dark:text-stone-400 text-sm mb-12">
            {t.updated}
          </p>

          <div className="space-y-10">
            {t.sections.map((section) => (
              <section key={section.heading}>
                <h2 className="text-xl font-bold text-stone-900 dark:text-stone-100 mb-3">
                  {section.heading}
                </h2>
                <p className="text-stone-600 dark:text-stone-400 leading-relaxed">
                  {section.body}
                </p>
              </section>
            ))}
          </div>
        </div>
      </div>

      <SiteFooter lang={lang} labels={t.footer} />
    </HomeLayout>
  );
}
