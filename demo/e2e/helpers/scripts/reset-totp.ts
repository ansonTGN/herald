/**
 * Standalone TOTP Reset Script
 *
 * This script can be run from the command line to reset TOTP configurations.
 * Useful for quick cleanup during development and testing.
 *
 * Usage:
 *   npx tsx reset-totp.ts [command] [options]
 *
 * Commands:
 *   user <user-id>              Disable TOTP for a specific user
 *   user-email <email>          Disable TOTP for user by email
 *   realm <realm-id>            Disable TOTP for all users in realm
 *   realm-config <realm-id>     Disable realm TOTP settings
 *   admin                       Disable admin TOTP (email: admin@cas.com)
 *   reset <realm-id>            Full reset (disable all users and realm config)
 *   list <realm-id>             List users with TOTP enabled
 *   check <user-id>             Check user TOTP status
 *   status <realm-id>           Check realm TOTP settings
 *
 * Examples:
 *   npx tsx reset-totp.ts admin
 *   npx tsx reset-totp.ts user-email test@example.com
 *   npx tsx reset-totp.ts reset admin
 *   npx tsx reset-totp.ts list admin
 */

import {
  disableUserTOTP,
  disableRealmUserTOTP,
  disableRealmTOTP,
  getUserTOTPConfig,
  getRealmTOTPSettings,
  listUsersWithTOTPEnabled,
  resetRealmTOTP,
  disableAdminTOTPByEmail,
  closePool,
} from '../totp-db-helper.js'

// ============================================================================
// Command Line Interface
// ============================================================================

async function main() {
  const args = process.argv.slice(2)

  if (args.length === 0) {
    printUsage()
    process.exit(1)
  }

  const command = args[0]
  const param = args[1]

  try {
    switch (command) {
      case 'user':
        if (!param) {
          console.error('Error: user-id is required for "user" command')
          process.exit(1)
        }
        await handleDisableUser(param)
        break

      case 'user-email':
        if (!param) {
          console.error('Error: email is required for "user-email" command')
          process.exit(1)
        }
        await handleDisableUserByEmail(param)
        break

      case 'realm':
        if (!param) {
          console.error('Error: realm-id is required for "realm" command')
          process.exit(1)
        }
        await handleDisableRealm(param)
        break

      case 'realm-config':
        await handleDisableRealmConfig(param || 'admin')
        break

      case 'admin':
        await handleDisableAdmin()
        break

      case 'reset':
        if (!param) {
          console.error('Error: realm-id is required for "reset" command')
          process.exit(1)
        }
        await handleResetRealm(param)
        break

      case 'list':
        if (!param) {
          console.error('Error: realm-id is required for "list" command')
          process.exit(1)
        }
        await handleListUsers(param)
        break

      case 'check':
        if (!param) {
          console.error('Error: user-id is required for "check" command')
          process.exit(1)
        }
        await handleCheckUser(param)
        break

      case 'status':
        await handleRealmStatus(param || 'admin')
        break

      default:
        console.error(`Unknown command: ${command}`)
        printUsage()
        process.exit(1)
    }
  } catch (error) {
    console.error('Error:', error)
    process.exit(1)
  } finally {
    await closePool()
  }
}

// ============================================================================
// Command Handlers
// ============================================================================

async function handleDisableUser(userId: string): Promise<void> {
  console.log(`Disabling TOTP for user: ${userId}`)
  await disableUserTOTP(userId)
  console.log('✓ User TOTP disabled')
}

async function handleDisableUserByEmail(email: string): Promise<void> {
  console.log(`Disabling TOTP for user email: ${email}`)
  await disableAdminTOTPByEmail(email)
  console.log('✓ User TOTP disabled')
}

async function handleDisableRealm(realmId: string): Promise<void> {
  console.log(`Disabling TOTP for all users in realm: ${realmId}`)
  const count = await disableRealmUserTOTP(realmId)
  console.log(`✓ Disabled TOTP for ${count} users`)
}

async function handleDisableRealmConfig(realmId: string): Promise<void> {
  console.log(`Disabling realm TOTP configuration for: ${realmId}`)
  await disableRealmTOTP(realmId)
  console.log('✓ Realm TOTP configuration disabled')
}

async function handleDisableAdmin(): Promise<void> {
  console.log('Disabling admin TOTP')
  await disableAdminTOTPByEmail('admin@cas.com')
  console.log('✓ Admin TOTP disabled')
}

async function handleResetRealm(realmId: string): Promise<void> {
  console.log(`Resetting TOTP for realm: ${realmId}`)
  await resetRealmTOTP(realmId)
  console.log('✓ Realm TOTP reset complete')
}

async function handleListUsers(realmId: string): Promise<void> {
  console.log(`Listing users with TOTP enabled in realm: ${realmId}`)
  const userIds = await listUsersWithTOTPEnabled(realmId)

  if (userIds.length === 0) {
    console.log('No users have TOTP enabled')
  } else {
    console.log(`Found ${userIds.length} users with TOTP enabled:`)
    userIds.forEach((userId) => console.log(`  - ${userId}`))
  }
}

async function handleCheckUser(userId: string): Promise<void> {
  console.log(`Checking TOTP status for user: ${userId}`)
  const config = await getUserTOTPConfig(userId)

  if (!config) {
    console.log('User does not have TOTP configuration')
  } else {
    console.log(`TOTP Status:`)
    console.log(`  Enabled: ${config.enabled}`)
    console.log(`  Verified At: ${config.verified_at || 'Not verified'}`)
    console.log(`  Last Used: ${config.last_used_at || 'Never'}`)
  }
}

async function handleRealmStatus(realmId: string): Promise<void> {
  console.log(`Checking realm TOTP settings for: ${realmId}`)
  const settings = await getRealmTOTPSettings(realmId)

  console.log(`Realm TOTP Settings:`)
  console.log(`  Enabled: ${settings?.enabled}`)
  console.log(`  Force Enabled: ${settings?.force_enabled}`)
}

// ============================================================================
// Usage
// ============================================================================

function printUsage(): void {
  console.log(`
TOTP Reset Script
=================

Usage: npx tsx reset-totp.ts [command] [options]

Commands:
  user <user-id>              Disable TOTP for a specific user
  user-email <email>          Disable TOTP for user by email
  realm <realm-id>            Disable TOTP for all users in realm
  realm-config <realm-id>     Disable realm TOTP settings
  admin                       Disable admin TOTP (email: admin@cas.com)
  reset <realm-id>            Full reset (disable all users and realm config)
  list <realm-id>             List users with TOTP enabled
  check <user-id>             Check user TOTP status
  status <realm-id>           Check realm TOTP settings

Examples:
  npx tsx reset-totp.ts admin
  npx tsx reset-totp.ts user-email test@example.com
  npx tsx reset-totp.ts reset admin
  npx tsx reset-totp.ts list admin

Environment Variables:
  DATABASE_URL                PostgreSQL connection string
                             (default: postgres://postgres:postgres@127.0.0.1:5432/herald_demo)
`)
}

// ============================================================================
// Main Execution
// ============================================================================

main().catch(console.error)
