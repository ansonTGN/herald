/**
 * Demo 测试数据统一管理
 *
 * 本文件集中管理所有 Demo 测试使用的测试数据，包括：
 * - 测试 Realm 数据
 * - 测试角色数据
 * - 数据完整性验证规范（Data Integrity Specs）
 * - 辅助函数（生成测试数据）
 *
 * ✅ Cleaned: Removed unused exports (RBAC_TEST_DATA, ADMIN_ACCOUNT, DEFAULT_TEST_USER, etc.)
 */

// 重新导出 auth.ts 中的测试数据
export { REALM_ADMINS, DEMO_USERS } from '../helpers/auth'

// 同时导入供内部使用
import { REALM_ADMINS } from '../helpers/auth'

/**
 * 数据完整性验证规范
 *
 * 用于 verifyTestEnvironment() 的数据完整性验证。
 * 注意：默认情况下不启用，需要显式指定 validateDataIntegrity: true。
 */
export const TEST_INTEGRITY_SPECS = {
  /**
   * Admin Realm 数据完整性规范
   */
  adminRealm: {
    realms: [
      {
        realmId: 'admin',
        expectedName: 'Admin',
      },
    ],
    users: [
      {
        email: 'admin@cas.com',
        realmId: 'admin',
        expectedNickname: 'admin',
        expectedStatus: 1, // normal
        expectedRoles: ['realm-admin'],
      },
    ],
    requiredRoles: [
      {
        roleName: 'realm-admin',
        realmId: 'admin',
      },
      {
        roleName: 'user',
        realmId: 'admin',
      },
    ],
  },

  /**
   * Realm1 数据完整性规范
   */
  realm1: {
    realms: [
      {
        realmId: 'realm1',
        expectedName: 'Realm 1',
      },
    ],
    users: [
      {
        email: 'realm1-admin@test.com',
        realmId: 'realm1',
        expectedRoles: ['realm-admin'],
      },
    ],
    requiredRoles: [
      {
        roleName: 'realm-admin',
        realmId: 'realm1',
      },
    ],
  },

  /**
   * Realm2 数据完整性规范
   */
  realm2: {
    realms: [
      {
        realmId: 'realm2',
        expectedName: 'Realm 2',
      },
    ],
    users: [
      {
        email: 'realm2-admin@test.com',
        realmId: 'realm2',
        expectedRoles: ['realm-admin'],
      },
    ],
    requiredRoles: [
      {
        roleName: 'realm-admin',
        realmId: 'realm2',
      },
    ],
  },
}

// Re-export types from environment-setup for convenience
export type {
  DataIntegritySpec,
  RealmValidationSpec,
  RoleValidationSpec,
  UserValidationSpec,
  VerifyEnvironmentOptions,
} from '../helpers/environment-setup'

/**
 * 测试账号类型
 */
export interface TestAccount {
  email: string
  password: string
  realmId: string
}

/**
 * 测试 Realm 数据
 */
export interface TestRealm {
  id: string
  name: string
  adminEmail: string
}

/**
 * Demo 测试中使用的 Realm 数据
 */
export const TEST_REALMS: Record<string, TestRealm> = {
  admin: {
    id: 'admin',
    name: 'Admin',
    adminEmail: 'admin@cas.com',
  },
  realm1: {
    id: 'realm1',
    name: 'Realm 1',
    adminEmail: 'realm1-admin@test.com',
  },
  realm2: {
    id: 'realm2',
    name: 'Realm 2',
    adminEmail: 'realm2-admin@test.com',
  },
}

/**
 * Demo 测试中使用的角色数据
 */
export const TEST_ROLES = {
  USER: 'user',
  ADMIN: 'admin',
  REALM_ADMIN: 'realm-admin',
} as const

/**
 * 生成测试用户数据
 *
 * @param options 用户选项
 * @returns 测试用户数据
 */
export function generateTestUser(options?: {
  email?: string
  password?: string
  nickname?: string
  realmId?: string
}): TestAccount & { nickname: string } {
  const timestamp = Date.now()
  const random = Math.floor(Math.random() * 1000)

  return {
    email: options?.email || `test-user-${timestamp}-${random}@demo.com`,
    password: options?.password || 'password123',
    nickname: options?.nickname || `Test User ${timestamp}`,
    realmId: options?.realmId || 'admin',
  }
}
