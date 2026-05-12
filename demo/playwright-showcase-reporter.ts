import { FullConfig, FullResult, Reporter, Suite, TestCase, TestResult } from '@playwright/test'
import * as path from 'path'
import * as fs from 'fs'
import * as os from 'os'

/**
 * Showcase Reporter for Herald Screenshot Gallery
 *
 * 自动收集和整理演示测试的截图，生成结构化的展示目录和元数据清单
 */

interface ScreenshotAttachment {
  path: string
  name: string
  contentType: string
}

interface ScreenshotMetadata {
  id: string
  path: string
  title: string
  description: string
  component: string
  route?: string
  tags: string[]
  viewport?: string
  timestamp: string
}

interface UserStoryMetadata {
  id: string
  name: string
  description: string
  screenshots: ScreenshotMetadata[]
}

interface RoleMetadata {
  id: string
  name: string
  description: string
  icon: string
  stories: UserStoryMetadata[]
}

interface ShowcaseManifest {
  version: string
  generated: string
  totalScreenshots: number
  roles: RoleMetadata[]
}

class ShowcaseReporter implements Reporter {
  private config!: FullConfig
  private screenshotsByRole = new Map<string, RoleMetadata>()
  private screenshotCounter = 0

  onBegin(config: FullConfig, suite: Suite) {
    this.config = config
    console.log(`[Showcase Reporter] Starting screenshot collection...`)
  }

  onTestEnd(test: TestCase, result: TestResult) {
    // 从测试文件路径提取角色信息
    const match = test.location.file.match(/demo[\/\\](.+?)[\/\\].*\.e2e\.ts$/)
    if (!match) {
      // 不是 demo 测试，跳过
      return
    }

    const rolePath = match[1]
    const roleInfo = this.parseRole(rolePath)

    // 从测试标题提取用户故事
    const storyInfo = this.parseUserStory(test.title)

    // 收集截图
    const screenshots = this.collectScreenshots(test, result, roleInfo.id, storyInfo.id)

    if (screenshots.length === 0) {
      return
    }

    // 更新角色和用户故事的元数据
    this.updateMetadata(roleInfo, storyInfo, screenshots)
  }

  onEnd(result: FullResult) {
    console.log(`[Showcase Reporter] Collecting ${this.screenshotCounter} screenshots...`)

    // 生成 manifest.json
    const manifest = this.generateManifest()

    // 保存 manifest
    this.saveManifest(manifest)

    // 整理截图到结构化目录
    this.organizeScreenshots(manifest)

    console.log(`[Showcase Reporter] ✓ Complete! Generated ${manifest.totalScreenshots} screenshots`)
  }

  private parseRole(rolePath: string): { id: string; name: string; description: string; icon: string } {
    const roleMap: Record<string, { name: string; description: string; icon: string }> = {
      'super-admin': {
        name: '主管理员',
        description: '跨Realm管理权限',
        icon: 'shield'
      },
      'realm-admin': {
        name: '次管理员',
        description: '管理单个Realm',
        icon: 'user-cog'
      },
      'regular-user': {
        name: '普通用户',
        description: '访问个人中心',
        icon: 'user'
      },
      'third-party-app': {
        name: '第三方应用',
        description: 'OAuth集成',
        icon: 'app-indicator'
      }
    }

    const roleKey = rolePath.replace(/[\/\\]/g, '-')
    const roleInfo = roleMap[roleKey] || {
      name: roleKey,
      description: `${roleKey}演示`,
      icon: 'circle'
    }

    return {
      id: roleKey,
      ...roleInfo
    }
  }

  private parseUserStory(testTitle: string): { id: string; name: string; description: string } {
    // 从测试标题中提取用户故事
    // 格式: "用户故事1：跨 Realm 用户管理 › 完整跨Realm用户管理流程"
    const parts = testTitle.split(' › ')
    const storyPart = parts[0] || testTitle

    // 提取故事 ID（如 "user-management"）
    const storyId = this.slugify(storyPart.replace(/用户故事\d+：|›/g, '').trim())

    return {
      id: storyId,
      name: storyPart,
      description: `${storyPart}的完整演示流程`
    }
  }

  private collectScreenshots(
    test: TestCase,
    result: TestResult,
    roleId: string,
    storyId: string
  ): ScreenshotMetadata[] {
    const screenshots: ScreenshotMetadata[] = []

    // 收集所有截图附件
    for (const attachment of result.attachments) {
      if (attachment.name !== 'screenshot' || !attachment.path) {
        continue
      }

      this.screenshotCounter++
      const screenshotId = `${String(this.screenshotCounter).padStart(2, '0')}`

      // 从附件路径提取信息
      const parsedPath = path.parse(attachment.path)
      const componentType = this.inferComponentType(attachment.path, test.title)

      screenshots.push({
        id: screenshotId,
        path: attachment.path,
        title: this.generateScreenshotTitle(test.title, screenshotId),
        description: this.generateScreenshotDescription(test.title, screenshotId),
        component: componentType.type,
        route: componentType.route,
        tags: this.generateTags(test.title, componentType.type),
        viewport: '1920x1080',
        timestamp: new Date(result.startTime).toISOString()
      })
    }

    return screenshots
  }

  private inferComponentType(
    screenshotPath: string,
    testTitle: string
  ): { type: string; route?: string } {
    const title = testTitle.toLowerCase()
    const path = screenshotPath.toLowerCase()

    // 从测试标题推断组件类型
    if (title.includes('登录') || title.includes('login')) {
      return { type: 'page', route: '/$realm_id/login' }
    }
    if (title.includes('注册') || title.includes('register')) {
      return { type: 'page', route: '/$realm_id/register' }
    }
    if (title.includes('用户列表') || title.includes('user list')) {
      return { type: 'datatable', route: '/$realm_id/users' }
    }
    if (title.includes('创建用户') || title.includes('编辑用户') || title.includes('dialog')) {
      return { type: 'dialog', route: '/$realm_id/users' }
    }
    if (title.includes('角色') || title.includes('role')) {
      if (title.includes('列表') || title.includes('list')) {
        return { type: 'datatable', route: '/$realm_id/permission/role' }
      }
      return { type: 'dialog', route: '/$realm_id/permission/role' }
    }
    if (title.includes('客户端') || title.includes('client')) {
      return { type: 'datatable', route: '/$realm_id/clients' }
    }
    if (title.includes('设置') || title.includes('settings')) {
      return { type: 'page', route: '/$realm_id/settings' }
    }
    if (title.includes('个人资料') || title.includes('profile')) {
      return { type: 'page', route: '/$realm_id/profile/info' }
    }
    if (title.includes('仪表盘') || title.includes('dashboard')) {
      return { type: 'page', route: '/$realm_id/dashboard' }
    }

    // 默认为页面
    return { type: 'page' }
  }

  private generateScreenshotTitle(testTitle: string, id: string): string {
    // 基于测试标题生成截图标题
    const action = testTitle.split(' › ').pop() || testTitle
    return `${id}. ${action}`
  }

  private generateScreenshotDescription(testTitle: string, id: string): string {
    const parts = testTitle.split(' › ')
    if (parts.length > 1) {
      return `${parts[0]} - ${parts[1]}`
    }
    return testTitle
  }

  private generateTags(testTitle: string, componentType: string): string[] {
    const tags = [componentType]
    const title = testTitle.toLowerCase()

    if (title.includes('登录') || title.includes('login')) tags.push('authentication')
    if (title.includes('用户') || title.includes('user')) tags.push('user-management')
    if (title.includes('权限') || title.includes('permission') || title.includes('角色')) tags.push('permission')
    if (title.includes('客户端') || title.includes('client')) tags.push('oauth')
    if (title.includes('设置') || title.includes('settings')) tags.push('configuration')

    return tags
  }

  private updateMetadata(
    roleInfo: { id: string; name: string; description: string; icon: string },
    storyInfo: { id: string; name: string; description: string },
    screenshots: ScreenshotMetadata[]
  ) {
    // 获取或创建角色元数据
    if (!this.screenshotsByRole.has(roleInfo.id)) {
      this.screenshotsByRole.set(roleInfo.id, {
        id: roleInfo.id,
        name: roleInfo.name,
        description: roleInfo.description,
        icon: roleInfo.icon,
        stories: []
      })
    }

    const roleMetadata = this.screenshotsByRole.get(roleInfo.id)!

    // 查找或创建用户故事
    let storyMetadata = roleMetadata.stories.find(s => s.id === storyInfo.id)
    if (!storyMetadata) {
      storyMetadata = {
        id: storyInfo.id,
        name: storyInfo.name,
        description: storyInfo.description,
        screenshots: []
      }
      roleMetadata.stories.push(storyMetadata)
    }

    // 添加截图
    storyMetadata.screenshots.push(...screenshots)
  }

  private generateManifest(): ShowcaseManifest {
    const roles: RoleMetadata[] = []

    for (const [, roleMetadata] of this.screenshotsByRole) {
      roles.push(roleMetadata)
    }

    return {
      version: '1.0.0',
      generated: new Date().toISOString(),
      totalScreenshots: this.screenshotCounter,
      roles
    }
  }

  private saveManifest(manifest: ShowcaseManifest) {
    const outputDir = path.join(this.config.rootDir, 'playwright-report')
    const manifestPath = path.join(outputDir, 'manifest.json')

    fs.mkdirSync(outputDir, { recursive: true })
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2), 'utf-8')

    console.log(`[Showcase Reporter] ✓ Manifest saved to: ${manifestPath}`)
  }

  private organizeScreenshots(manifest: ShowcaseManifest) {
    const screenshotsBaseDir = path.join(this.config.rootDir, 'playwright-report', 'screenshots')

    // 创建目标目录结构
    for (const role of manifest.roles) {
      for (const story of role.stories) {
        const storyDir = path.join(screenshotsBaseDir, role.id, story.id)
        fs.mkdirSync(storyDir, { recursive: true })
      }
    }

    // 复制和重命名截图
    let copiedCount = 0
    for (const role of manifest.roles) {
      for (const story of role.stories) {
        for (const screenshot of story.screenshots) {
          const sourcePath = screenshot.path
          const targetDir = path.join(screenshotsBaseDir, role.id, story.id)
          const targetFileName = `${screenshot.id}.png`
          const targetPath = path.join(targetDir, targetFileName)

          try {
            if (fs.existsSync(sourcePath)) {
              fs.copyFileSync(sourcePath, targetPath)
              // 更新路径为相对路径
              screenshot.path = `/screenshots/${role.id}/${story.id}/${targetFileName}`
              copiedCount++
            }
          } catch (error) {
            console.warn(`[Showcase Reporter] Warning: Failed to copy screenshot: ${sourcePath}`, error)
          }
        }
      }
    }

    // 重新保存更新了路径的 manifest
    const manifestPath = path.join(this.config.rootDir, 'playwright-report', 'manifest.json')
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2), 'utf-8')

    console.log(`[Showcase Reporter] ✓ Organized ${copiedCount} screenshots in: ${screenshotsBaseDir}`)
  }

  private slugify(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '') // 移除特殊字符
      .replace(/\s+/g, '-')      // 空格替换为连字符
      .replace(/-+/g, '-')       // 多个连字符合并为一个
      .trim()
  }
}

export default ShowcaseReporter
