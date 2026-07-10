import { HomeLayout } from "fumadocs-ui/layouts/home";
import { baseOptions, SiteFooter } from "@/lib/layout.shared";

interface TermsTexts {
  title: string;
  updated: string;
  sections: { heading: string; body: string }[];
  footer: {
    copyright: string;
    privacy: string;
    terms: string;
  };
}

const en: TermsTexts = {
  title: "Terms of Service",
  updated: "Last updated: July 2026",
  sections: [
    {
      heading: "License",
      body: "Herald is released under the Apache License 2.0. You are free to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the software, subject to the following conditions: The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.",
    },
    {
      heading: "Disclaimer",
      body: 'THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.',
    },
    {
      heading: "Website",
      body: "This website provides information about the Herald project. The content is for informational purposes only. We make no guarantees about the accuracy or completeness of the information presented.",
    },
    {
      heading: "Third-Party Dependencies",
      body: "Herald integrates with and depends on various third-party libraries and services (e.g., Stripe, OAuth providers, database drivers). Each third-party component is governed by its own license and terms of service. You are responsible for reviewing and complying with those terms when using Herald.",
    },
    {
      heading: "Contributions",
      body: "By contributing to the Herald project (e.g., submitting pull requests, issues, or discussions on GitHub), you agree that your contributions will be licensed under the same Apache License 2.0 that covers the project.",
    },
    {
      heading: "Changes",
      body: "We may update these terms from time to time. Any changes will be reflected on this page with an updated revision date. The latest version is always available in our GitHub repository.",
    },
    {
      heading: "Contact",
      body: "If you have questions about these terms, please open an issue on our GitHub issue tracker.",
    },
  ],
  footer: {
    copyright: "Herald · Apache 2.0",
    privacy: "Privacy",
    terms: "Terms",
  },
};

const zh: TermsTexts = {
  title: "服务条款",
  updated: "最后更新：2026 年 7 月",
  sections: [
    {
      heading: "许可证",
      body: "Herald 采用 Apache License 2.0 发布。你可以自由使用、复制、修改、合并、发布、分发、再许可和/或销售本软件副本，但须满足以下条件：上述版权声明和本许可声明必须包含在本软件的所有副本或实质性部分中。",
    },
    {
      heading: "免责声明",
      body: '本软件按"原样"提供，不提供任何明示或暗示的担保，包括但不限于对适销性、特定用途适用性和非侵权性的担保。在任何情况下，作者或版权持有人均不对任何索赔、损害或其他责任负责，无论是合同、侵权还是其他行为，因本软件或使用或其他交易而引起、产生或与之相关。',
    },
    {
      heading: "网站",
      body: "本网站提供有关 Herald 项目的信息。内容仅供参考。我们不保证所呈现信息的准确性或完整性。",
    },
    {
      heading: "第三方依赖",
      body: "Herald 集成并依赖各种第三方库和服务（例如 Stripe、OAuth 提供商、数据库驱动程序）。每个第三方组件受其自身的许可证和服务条款约束。在使用 Herald 时，你有责任查看并遵守这些条款。",
    },
    {
      heading: "贡献",
      body: "通过向 Herald 项目做出贡献（例如在 GitHub 上提交 pull request、issue 或讨论），你同意你的贡献将采用与项目相同的 Apache License 2.0 进行许可。",
    },
    {
      heading: "变更",
      body: "我们可能会不时更新这些条款。任何变更都会在本页面显示更新后的修订日期。最新版本始终可在我们的 GitHub 仓库中获取。",
    },
    {
      heading: "联系我们",
      body: "如果你对这些条款有疑问，请在 GitHub issue 跟踪器中提交 issue。",
    },
  ],
  footer: {
    copyright: "Herald · Apache 2.0",
    privacy: "隐私政策",
    terms: "服务条款",
  },
};

const textsMap: Record<string, TermsTexts> = { en, zh };

export function TermsPage({ lang }: { lang: string }) {
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
