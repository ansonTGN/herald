/**
 * Centralized Selectors for Herald Frontend E2E Tests
 *
 * Page Object Model (POM) pattern:
 * - All selectors defined in one place
 * - Easy maintenance when frontend changes
 * - Consistent selector usage across tests
 *
 * Selector Priority:
 * 1. data-testid (most stable)
 * 2. Aria roles (semantic)
 * 3. Text content (fallback)
 */

export const SELECTORS = {
  /**
   * Login Page Selectors
   * Route: /{realmId}/auth/login
   */
  login: {
    container: '[data-testid="login-card"], [data-testid="login-container"]',
    title: '[data-testid="login-title"]',
    usernameInput: '[data-testid="email-input"]',
    emailInput: '[data-testid="email-input"]',
    passwordInput: '[data-testid="password-input"]',
    submitButton: '[data-testid="login-submit-button"]',
    errorMessage: '[data-testid="login-error-message"]',
  },

  /**
   * Dashboard Selectors
   * Route: /{realmId}/
   */
  dashboard: {
    heading: 'h1:has-text("Dashboard")',
    statsRow: '[data-testid="dashboard-stats-row"]',
    totalUsersCard: '[data-testid="dashboard-total-users-card"]',
    newUsersCard: '[data-testid="dashboard-new-users-card"]',
    activeUsersCard: '[data-testid="dashboard-active-users-card"]',
    authTrendChart: '[data-testid="dashboard-auth-trend-chart"]',
    quickNav: '[data-testid="dashboard-quick-nav"]',
    quickNavUsers: '[data-testid="dashboard-users-card"]',
    quickNavRoles: '[data-testid="dashboard-roles-card"]',
    quickNavPermissions: '[data-testid="dashboard-permissions-card"]',
    quickNavClientApps: '[data-testid="dashboard-client-apps-card"]',
    quickNavRealms: '[data-testid="dashboard-realms-card"]',
    quickNavSettings: '[data-testid="dashboard-settings-card"]',
    errorState: '[data-testid="dashboard-error"]',
    retryButton: '[data-testid="dashboard-retry-button"]',
    chartSkeleton: '[data-testid="dashboard-chart-skeleton"]',
  },

  /**
   * Sidebar Selectors
   */
  sidebar: {
    container: '[data-testid="admin-sidebar"]',
    menuDashboard: '[data-testid="sidebar-menu-dashboard"]',
    menuUsers: '[data-testid="sidebar-menu-users"]',
    menuRoles: '[data-testid="sidebar-menu-roles"]',
    menuSettings: '[data-testid="sidebar-menu-settings"]',
    menuRealms: '[data-testid="sidebar-menu-realms"]',
    menuClientApps: '[data-testid="sidebar-menu-client-apps"]',
    menuAuthorization: '[data-testid="sidebar-menu-authorization"]',
    menuPermissions: '[data-testid="sidebar-menu-permissions"]',
    menuApiKeys: '[data-testid="sidebar-menu-api-keys"]',
    menuAuditLog: '[data-testid="sidebar-menu-audit-log"]',
    menuEntitlementMappings: '[data-testid="sidebar-menu-entitlement-mappings"]',
  },

  /**
   * Language Switcher Selectors
   */
  languageSwitcher: {
    trigger: '[data-testid="language-switcher"]',
    enItem: '[data-testid="language-switcher-item-en"]',
    zhItem: '[data-testid="language-switcher-item-zh-CN"]',
  },

  /**
   * Header Selectors
   */
  header: {
    container: '[data-testid="admin-header"]',
    userAvatar: '[data-testid="user-avatar"]',
    logoutButton: '[data-testid="logout-menu-item"]',
    userMenu: '[data-testid="user-menu"]',
    userDisplayName: '[data-testid="user-display-name"]',
  },

  /**
   * Audit Page Selectors
   * Route: /{realmId}/manage/audit
   */
  audit: {
    container: '[data-testid="audit-page"]',
    heading: '[data-testid="audit-heading"]',
    table: '[data-testid="audit-table"]',
    filterBar: '[data-testid="audit-filter-bar"]',
    filterCategory: '[data-testid="audit-category-select"]',
    filterAction: '[data-testid="audit-action-select"]',
    filterActorId: '[data-testid="audit-actor-input"]',
    filterStartDate: '[data-testid="audit-start-date-input"]',
    filterEndDate: '[data-testid="audit-end-date-input"]',
    filterClear: '[data-testid="audit-clear-filters-button"]',
    tableLoading: '[data-testid="audit-table-loading"]',
    detailSheet: '[data-testid="audit-detail-sheet"]',
    detailError: '[data-testid="audit-detail-error"]',
    detailClose: '[data-testid="audit-detail-close-button"]',
    detailJson: '[data-testid="audit-detail-json"]',
    detailResult: '[data-testid="audit-detail-result"]',
    pagination: '[data-testid="audit-pagination"]',
    paginationPrevious: '[data-testid="audit-pagination-previous"]',
    paginationNext: '[data-testid="audit-pagination-next"]',
  },

  /**
   * Users Page Selectors
   * Route: /{realmId}/users
   */
  users: {
    container: '[data-testid="users-page"]',
    heading: '[data-testid="users-heading"]',
    table: '[data-testid="users-table"]',
    addButton:
      '[data-testid="add-user-button"], [data-testid="create-user-button"]',
    searchInput: '[data-testid="users-search-input"]',
    roleCheckbox: '[data-testid="user-create-role-checkbox"]',
    deleteDialog: '[data-testid="delete-user-dialog"]',
    confirmDeleteButton: '[data-testid="confirm-delete-user-button"]',
  },

  /**
   * Reset Password Dialog Selectors
   * Triggered from: /{realmId}/manage/users (user table row action)
   *
   * The row-level reset password button uses a dynamic testid
   * (user-table-{row.index}-reset-password-button) and is constructed
   * per-row in the POM via row-relative locator.
   */
  resetPassword: {
    confirmDialog: '[data-testid="reset-password-dialog"]',
    confirmButton: '[data-testid="confirm-reset-password-button"]',
    resultDialog: '[data-testid="reset-password-result-dialog"]',
    newPasswordText: '[data-testid="new-password-text"]',
    copyButton: '[data-testid="copy-password-button"]',
  },

  /**
   * Roles Page Selectors
   * Route: /{realmId}/roles
   */
  roles: {
    container: '[data-testid="roles-page"]',
    heading: '[data-testid="roles-heading"]',
    table: '[data-testid="role-table"]',
    addButton: '[data-testid="role-create-button"]',
    permissionsButton: '[data-testid="permissions-button"]',
    // Create dialog
    createNameInput: '[data-testid="role-create-name-input"]',
    createDescriptionInput: '[data-testid="role-create-description-input"]',
    createSubmitButton: '[data-testid="role-create-submit-button"]',
    // Edit dialog
    editNameInput: '[data-testid="role-edit-name-input"]',
    editDescriptionInput: '[data-testid="role-edit-description-input"]',
    editSubmitButton: '[data-testid="role-edit-submit-button"]',
  },

  /**
   * Permissions Page Selectors
   * Route: /{realmId}/permissions
   */
  permissions: {
    container: '[data-testid="permissions-page"]',
    heading: '[data-testid="permissions-heading"]',
    table: '[data-testid="permissions-table"]',
    addButton: '[data-testid="permission-create-button"]',
    // Create dialog
    createNameInput: '[data-testid="permission-create-name-input"]',
    createDescriptionInput:
      '[data-testid="permission-create-description-input"]',
    createSubmitButton: '[data-testid="permission-create-submit-button"]',
    // Edit dialog
    editNameInput: '[data-testid="permission-edit-name-input"]',
    editDescriptionInput: '[data-testid="permission-edit-description-input"]',
    editSubmitButton: '[data-testid="permission-edit-submit-button"]',
  },

  /**
   * Realms Page Selectors
   * Route: /admin/realms
   */
  realms: {
    container: '[data-testid="realms-page"]',
    heading: '[data-testid="realms-heading"]',
    table: '[data-testid="realms-table"]',
    addButton:
      '[data-testid="add-realm-button"], [data-testid="create-realm-button"]',
  },

  /**
   * Profile Page Selectors
   * Route: /{realmId}/profile
   */
  profile: {
    container: '[data-testid="profile-page"]',
    heading: '[data-testid="profile-heading"]',
    // Profile sidebar
    sidebarContainer: '[data-testid="profile-sidebar"]',
    menuProfile: '[data-testid="profile-menu-profile"]',
    menuSecurity: '[data-testid="profile-menu-security"]',
    logoutButton: '[data-testid="profile-logout-button"]',
    headerContainer: '[data-testid="profile-header"]',
    // Profile fields
    emailField: '[data-testid="profile-email"]',
    nicknameField: '[data-testid="profile-nickname"]',
    nicknameInput: '[data-testid="nickname-input"]',
    statusField: '[data-testid="profile-status"]',
    saveButton: '[data-testid="save-profile-button"]',
    // Change Password
    changePasswordHeading: '[data-testid="change-password-heading"]',
    oldPasswordInput: '[data-testid="change-password-old-input"]',
    newPasswordInput: '[data-testid="change-password-new-input"]',
    confirmPasswordInput: '[data-testid="change-password-confirm-input"]',
    changePasswordSubmitButton: '[data-testid="change-password-submit-button"]',
  },

  /**
   * Security/Profile Selectors
   * Route: /{realmId}/user/security
   */
  security: {
    pageTitle: '[data-testid="security-page-title"]',
    passwordSectionTitle: '[data-testid="password-section-title"]',
    // TOTP Section
    totpSectionTitle: '[data-testid="totp-section-title"]',
    // TOTP Status Card
    totpStatusCard: '[data-testid="totp-status-card"]',
    totpStatusCardEnabled: '[data-testid="totp-status-card-enabled"]',
    totpEnableButton: '[data-testid="totp-enable-button"]',
    totpDisableButton: '[data-testid="totp-disable-button"]',
    totpRegenerateButton: '[data-testid="totp-regenerate-button"]',
    // TOTP Setup Page (3-step flow)
    // Route: /{realmId}/user/security/totp-setup
    totpSetupPage: '[data-testid="totp-setup-page"]',
    totpSetupPageTitle: '[data-testid="totp-setup-page-title"]',
    totpSetupPageDescription: '[data-testid="totp-setup-page-description"]',
    totpSetupBackToSecurity: '[data-testid="totp-setup-back-to-security"]',
    // Step 1: Password Confirmation
    totpSetupStepPassword: '[data-testid="totp-setup-step-password"]',
    totpSetupPasswordInput: '[data-testid="totp-setup-password-input"]',
    totpSetupGenerateButton: '[data-testid="totp-setup-generate-button"]',
    totpPasswordError: '[data-testid="totp-password-error"]',
    // Step 2: QR Code Display
    totpSetupStepQRCode: '[data-testid="totp-setup-step-qr-code"]',
    totpQRCodeContainer: '[data-testid="totp-qr-code-container"]',
    totpQRCode: '[data-testid="totp-qr-code"]',
    totpSecretKey: '[data-testid="totp-qr-code-container"]',
    totpSetupBackButton: '[data-testid="totp-setup-back-button"]',
    totpSetupNextButton: '[data-testid="totp-setup-next-button"]',
    // Step 3: Verification
    totpSetupStepVerify: '[data-testid="totp-setup-step-verify"]',
    totpOtpInput: '[data-testid="totp-otp-input"]',
    totpOtpDigit: (index: number) => `[data-testid="totp-otp-digit-${index}"]`,
    totpVerifyBackButton: '[data-testid="totp-verify-back-button"]',
    totpVerifySubmitButton: '[data-testid="totp-verify-submit-button"]',
    totpVerifyLoading: '[data-testid="totp-verify-loading"]',
    // Common Setup Page Elements
    totpSavedBackupCodesCheckbox:
      '[data-testid="totp-saved-backup-codes-checkbox"]',
    totpSavedBackupCodesLabel: '[data-testid="totp-saved-backup-codes-label"]',
    // TOTP Disable Form (Dialog)
    totpDisablePasswordInput: '[data-testid="totp-disable-password-input"]',
    totpDisableCancelButton: '[data-testid="totp-disable-cancel-button"]',
    totpDisableSubmitButton: '[data-testid="totp-disable-submit-button"]',
    // TOTP Regenerate Form (Dialog)
    totpRegeneratePasswordInput:
      '[data-testid="totp-regenerate-password-input"]',
    totpRegenerateCancelButton: '[data-testid="totp-regenerate-cancel-button"]',
    // Backup Codes
    backupCodesCopyAllButton: '[data-testid="backup-codes-copy-all-button"]',
    backupCode: (index: number) => `[data-testid="backup-code-${index}"]`,
    // TOTP Status Display
    totpEnabledAt: '[data-testid="totp-enabled-at"]',
    totpLastVerifiedAt: '[data-testid="totp-last-verified-at"]',
    totpRemainingBackupCodes: '[data-testid="totp-remaining-backup-codes"]',
  },

  /**
   * Client Apps Page Selectors
   * Route: /{realmId}/manage/client-apps
   */
  clientApps: {
    page: '[data-testid="client-apps-page"]',
    heading: '[data-testid="client-apps-heading"]',
    table: '[data-testid="client-apps-table"]',
    addButton: '[data-testid="add-client-app-button"]',
    searchInput: '[data-testid="client-apps-search-input"]',
    // Table rows (by row index)
    rowByIndex: (index: number) => `[data-testid="client-app-row-${index}"]`,
    // Table rows (by app UUID)
    row: (appId: string) => `[data-app-id="${appId}"]`,
    // Table rows (by client ID)
    rowByClientId: (clientId: string) => `[data-client-id="${clientId}"]`,
    editButton: (appId: string) =>
      `[data-app-id="${appId}"] [data-testid="edit-client-app-button"]`,
    deleteButton: (appId: string) =>
      `[data-app-id="${appId}"] [data-testid="delete-client-app-button"]`,
    enabledSwitch: (appId: string) =>
      `[data-app-id="${appId}"] [data-testid="client-app-enabled-switch"]`,
  },

  /**
   * Client App Form Page Selectors
   * Dedicated page with tabs for creating/editing client apps (replaces dialog).
   */
  clientAppForm: {
    // Page container
    page: '[data-testid="client-app-form-page"]',
    pageTitle: '[data-testid="page-title"]',
    // Tab triggers
    tabBasic: '[data-testid="tab-basic"]',
    tabRedirectUris: '[data-testid="tab-redirect-uris"]',
    tabSecurity: '[data-testid="tab-security"]',
    tabAppearance: '[data-testid="tab-appearance"]',
    // Basic tab
    clientIdInput: '[data-testid="client-id-input"]',
    clientIdDisplay: '[data-testid="client-id-display"]',
    nameInput: '[data-testid="client-app-name-input"]',
    descriptionInput: '[data-testid="client-app-description-input"]',
    // Redirect URIs tab
    redirectUrisInput: '[data-testid="redirect-uris-input-field"]',
    // Security tab
    enabledSwitch: '[data-testid="client-app-enabled-switch"]',
    sessionTtlInput: '[data-testid="session-ttl-input"]',
    sessionTtlPreset: (label: string) =>
      `[data-testid="session-ttl-preset-${label}"]`,
    sessionRenewalTtlInput: '[data-testid="session-renewal-ttl-input"]',
    deviceCodeGrantSwitch: '[data-testid="device-code-grant-switch"]',
    regenerateSecretSwitch: '[data-testid="regenerate-secret-switch"]',
    // Appearance tab
    iconUrlInput: '[data-testid="icon-url-input"]',
    // Footer buttons
    cancelButton: '[data-testid="cancel-button"]',
    submitButton: '[data-testid="submit-button"]',
  },

  /**
   * API Keys Page Selectors
   * Route: /{realmId}/manage/api-keys
   */
  apiKeys: {
    page: '[data-testid="api-keys-page"]',
    heading: '[data-testid="api-keys-heading"]',
    table: '[data-testid="api-keys-table"]',
    addButton: '[data-testid="add-api-key-button"]',
    name: '[data-testid="api-key-name"]',
    enabledSwitch: '[data-testid="api-key-enabled-switch"]',
    statusBadge: '[data-testid="api-key-status-badge"]',
    expires: '[data-testid="api-key-expires"]',
    lastUsed: '[data-testid="api-key-last-used"]',
    clientApp: '[data-testid="api-key-client-app"]',
    editButton: '[data-testid="edit-api-key-button"]',
    deleteButton: '[data-testid="delete-api-key-button"]',
    rolesCell: '[data-testid="api-key-roles-cell"]',
    rolesOverflow: '[data-testid="api-key-roles-overflow"]',
    manageRolesButton: '[data-testid="manage-api-key-roles-button"]',
  },

  /**
   * API Key Form Page Selectors
   * Route: /{realmId}/manage/api-keys/new, /{realmId}/manage/api-keys/{apiKeyId}/edit
   */
  apiKeyForm: {
    page: '[data-testid="api-key-form-page"]',
    pageTitle: '[data-testid="page-title"]',
    backButton: '[data-testid="api-key-form-back-button"]',
    nameInput: '[data-testid="api-key-name-input"]',
    clientAppSelectorTrigger: '[data-testid="client-app-selector-trigger"]',
    clientAppSelectorSearch: '[data-testid="client-app-selector-search"]',
    clientAppSelectorDefault: '[data-testid="client-app-selector-default"]',
    clientAppSelectorItem: (appId: string) =>
      `[data-testid="client-app-selector-item-${appId}"]`,
    enabledSwitch: '[data-testid="api-key-enabled-switch"]',
    expiresAtInput: '[data-testid="api-key-expires-at-input"]',
    expiresAtClearButton: '[data-testid="api-key-expires-at-clear-button"]',
    cancelButton: '[data-testid="cancel-button"]',
    submitButton: '[data-testid="submit-button"]',
  },

  /**
   * API Key Reveal Page Selectors
   * Route: /{realmId}/manage/api-keys/reveal
   */
  apiKeyReveal: {
    page: '[data-testid="api-key-reveal-page"]',
    pageTitle: '[data-testid="page-title"]',
    backButton: '[data-testid="api-key-reveal-back-button"]',
    keyValue: '[data-testid="api-key-reveal-value"]',
    copyButton: '[data-testid="copy-api-key-button"]',
    doneButton: '[data-testid="api-key-reveal-done-button"]',
  },

  /**
   * API Key Delete Dialog Selectors
   */
  apiKeyDelete: {
    dialog: '[data-testid="delete-confirmation-dialog"]',
    cancelButton: '[data-testid="cancel-delete-button"]',
    confirmButton: '[data-testid="confirm-delete-button"]',
  },

  /**
   * API Key Roles Dialog Selectors
   */
  apiKeyRoles: {
    dialogContent: '[data-testid="api-key-roles-dialog-content"]',
    dialogTitle: '[data-testid="api-key-roles-dialog-title"]',
    dialogName: '[data-testid="api-key-roles-dialog-name"]',
    dialogClose: '[data-testid="api-key-roles-dialog-close"]',
    roleSelectorTrigger: '[data-testid="role-selector-trigger"]',
    roleSelectorSearch: '[data-testid="role-selector-search"]',
    roleSelectorItem: (roleId: string) =>
      `[data-testid="role-selector-item-${roleId}"]`,
  },

  /**
   * Common Component Selectors
   */
  common: {
    // Dialog/Modal
    dialog: '[data-testid="dialog"]',
    dialogTitle: '[data-testid="dialog-title"]',
    dialogContent: '[data-testid="dialog-content"]',
    dialogCloseButton: '[data-testid="dialog-close-button"]',
    dialogCancelButton: '[data-testid="dialog-cancel-button"]',
    dialogSubmitButton: '[data-testid="dialog-submit-button"]',

    // Form
    form: '[data-testid="form"]',
    formEmailInput: '[data-testid="email-input"]',
    formPasswordInput: '[data-testid="password-input"]',
    formNicknameInput: '[data-testid="nickname-input"]',
    formNameInput: '[data-testid="name-input"]',

    // Feedback
    // Sonner toast uses .data-[state=open]:animate-in to show toasts
    toast: '[data-testid="toast"], [data-sonner-toast]',
    toastMessage:
      '[data-testid="toast-message"], [data-sonner-toast] [data-description]',
    successMessage:
      '[data-testid="success-message"], [data-sonner-toast].success',
    errorMessage: '[data-testid="error-message"], [data-sonner-toast].error',

    // Loading
    loading: '[data-testid="loading"]',
    spinner: '[data-testid="spinner"]',
  },

  /**
   * Subscription History Page Selectors
   * Route: /{realmId}/manage/subscription-history
   */
  subscriptionHistory: {
    page: '[data-testid="subscription-history-page"]',
    filterContainer: '[data-testid="subscription-history-filter"]',
    listContainer: '[data-testid="subscription-history-list"]',
    backButton: '[data-testid="back-button"]',
    // Filter fields
    filterUserId: '[data-testid="filter-user-id"]',
    filterPlan: '[data-testid="filter-plan"]',
    filterEventType: '[data-testid="filter-event-type"]',
    filterStatus: '[data-testid="filter-status"]',
    filterFromDate: '[data-testid="filter-from-date"]',
    filterToDate: '[data-testid="filter-to-date"]',
    filterSortBy: '[data-testid="filter-sort-by"]',
    filterSortOrder: '[data-testid="filter-sort-order"]',
    // Filter buttons
    resetFilterButton: '[data-testid="reset-filter-button"]',
    applyFilterButton: '[data-testid="apply-filter-button"]',
    // History table
    historyRow: (index: number) => `[data-testid="history-row-${index}"]`,
    previousPageButton: '[data-testid="previous-page-button"]',
    nextPageButton: '[data-testid="next-page-button"]',
  },

  /**
   * Subscription Detail History Page Selectors
   * Route: /{realmId}/subscription/{subscriptionId}/history
   */
  subscriptionDetailHistory: {
    page: '[data-testid="subscription-detail-history-page"]',
    timelineContainer: '[data-testid="subscription-timeline"]',
    timelineLoading: '[data-testid="timeline-loading"]',
    timelineEmpty: '[data-testid="timeline-empty"]',
    backButton: '[data-testid="back-button"]',
    // Timeline events
    timelineEvent: (index: number) => `[data-testid="timeline-event-${index}"]`,
    viewEventDetailsButton: (index: number) =>
      `[data-testid="view-event-details-${index}"]`,
    // Event details dialog
    eventDetailDialog: '[data-testid="event-detail-dialog"]',
    // Event badges (parameterized by event type)
    eventBadge: (
      type:
        | "created"
        | "upgraded"
        | "downgraded"
        | "canceled"
        | "renewed"
        | "reactivated"
        | "expired",
    ) => `[data-testid="event-badge-${type}"]`,
  },

  /**
   * Billing Page Selectors
   * Route: /{realmId}/manage/billing
   */
  billing: {
    page: '[data-testid="billing-page"]',
    // Billing page navigation cards
    navEntitlementMappings: '[data-testid="billing-nav-entitlement-mappings"]',
    navSubscriptions: '[data-testid="billing-nav-subscriptions"]',
  },

  /**
   * Entitlement Mappings Page Selectors
   * Route: /{realmId}/manage/billing/entitlement-mappings
   */
  entitlementMappings: {
    page: '[data-testid="entitlement-mappings-page"]',
    heading: '[data-testid="entitlement-mappings-heading"]',
    table: '[data-testid="entitlement-mappings-table"]',
    providerFilterSelect: '[data-testid="provider-filter-select"]',
    mappingRow: (id: string) => `[data-testid="mapping-row-${id}"]`,
    firstMappingRow: () => '[data-testid^="mapping-row-"]',
    mappingEnabledToggle: (id: string) => `[data-testid="mapping-enabled-toggle-${id}"]`,
    providerSyncButton: '[data-testid="provider-sync-button"]',
    syncProviderSelect: '[data-testid="sync-provider-select"]',
    syncButton: '[data-testid="sync-button"]',
    // Detail dialog
    detailDialog: '[data-testid="entitlement-mapping-detail-dialog"]',
    entitlementKeyInput: '[data-testid="entitlement-key-input"]',
    pointsPerPeriodInput: '[data-testid="points-per-period-input"]',
    grantPeriodTypeSelect: '[data-testid="grant-period-type-select"]',
    validityDaysInput: '[data-testid="validity-days-input"]',
    grantOnSubscribeSwitch: '[data-testid="grant-on-subscribe-switch"]',
    maxPeriodsInput: '[data-testid="max-periods-input"]',
    mappingEnabledSwitch: '[data-testid="mapping-enabled-switch"]',
    saveMappingButton: '[data-testid="save-mapping-button"]',
    providerProductInfoCard: '[data-testid="provider-product-info-card"]',
    // Empty state
    emptyState: '[data-testid="entitlement-mappings-empty-state"]',
    // Pagination
    pagination: '[data-testid="entitlement-mappings-pagination"]',
  },

  /**
   * Admin Subscription List Page Selectors
   * Route: /{realmId}/manage/billing/subscriptions
   */
  adminSubscriptionList: {
    page: '[data-testid="admin-subscription-list-page"]',
    heading: '[data-testid="admin-subscription-list-heading"]',
    table: '[data-testid="admin-subscription-list-table"]',
    entitlementKeyFilterInput: '[data-testid="entitlement-key-filter-input"]',
    statusFilterSelect: '[data-testid="status-filter-select"]',
    paymentProviderFilterSelect: '[data-testid="payment-provider-filter-select"]',
    subscriptionRow: (id: string) => `[data-testid="subscription-row-${id}"]`,
    firstSubscriptionRow: () => '[data-testid^="subscription-row-"]',
    // Empty state
    emptyState: '[data-testid="admin-subscription-list-empty-state"]',
    // Pagination
    pagination: '[data-testid="admin-subscription-list-pagination"]',
  },

  /**
   * Points Management Page Selectors (Admin)
   * Route: /{realmId}/manage/points
   */
  pointsAdmin: {
    // Page containers
    accountsPage: '[data-testid="points-wallets-page"]',
    heading: 'h1:has-text("Points Management")',
    // User Wallets Section
    accountsSection: '[data-testid="points-wallets-page"]',
    accountsTable: '[data-testid="points-wallets-page"]',
    accountsSearch: '[data-testid="wallets-search-input"]',
    accountRow: (userId: string) => `[data-testid="wallet-row-${userId}"]`,
    firstAccountRow: () => '[data-testid^="wallet-row-"]',
    // Transaction History Section
    transactionsSection:
      '[data-testid="transaction-history-table"], [data-testid="no-transactions"]',
    transactionsTable: '[data-testid="transaction-history-table"]',
    transactionRow: (index: number) =>
      `[data-testid="transaction-row-${index}"]`,
    transactionType: (index: number) =>
      `[data-testid="transaction-type-${index}"]`,
    transactionAmount: (index: number) =>
      `[data-testid="transaction-amount-${index}"]`,
    transactionBalance: (index: number) =>
      `[data-testid="transaction-balance-${index}"]`,
    transactionDescription: (index: number) =>
      `[data-testid="transaction-description-${index}"]`,
    transactionTime: (index: number) =>
      `[data-testid="transaction-time-${index}"]`,
    // Transaction Filters
    transactionFilters: '[data-testid="transaction-filters"]',
    filterType: '[data-testid="filter-transaction-type"]',
    filterStartTime: '[data-testid="filter-from-date"]',
    filterEndTime: '[data-testid="filter-to-date"]',
    filterClientApp: '[data-testid="filter-client-app"]',
    resetFiltersButton: '[data-testid="clear-filters-button"]',
    applyFiltersButton: '[data-testid="apply-filters-button"]',
    // Credit-bucket admin wallets (US-CB cross-tenant bucket view):
    // each row keyed by (userId, bucketId). Bucket filter Select + optional
    // cross-bucket total card rendered above the list.
    walletRowByBucket: (userId: string, bucketId: string) =>
      `[data-testid="admin-wallet-row-${userId}-${bucketId}"]`,
    bucketFilter: '[data-testid="admin-wallets-bucket-filter"]',
    crossBucketTotal: '[data-testid="admin-wallets-cross-bucket-total"]',
  },
  /**
   * Points User Page Selectors
   * Route: /{realmId}/user/points
   */
  pointsUser: {
    page: '[data-testid="user-points-page"]',
    heading: 'h1:has-text("My Points")',
    // Balance Card
    balanceCard: '[data-testid="points-balance-card"]',
    balanceAmount: '[data-testid="points-balance"]',
    accountStatus: '[data-testid="points-wallet-status"]',
    // Credit-bucket (DE-D07): the bucket-aware UI renders one
    // `points-balance-card-${bucketId}` per held bucket (PointsBalanceCard.tsx).
    // The flat `points-balance-card` testid above only matches the loading
    // skeleton or a null-bucket fallback card. Sibling demos that just need to
    // assert "the user has at least one balance card rendered" (without
    // resolving a specific bucket UUID) use this prefix locator. Demos that
    // care about a SPECIFIC bucket use `balanceCardByBucket(bucketId)`.
    firstBalanceCard: '[data-testid^="points-balance-card-"]',
    // Bucket-grouped balances (credit-bucket US-CB-005). Per-bucket card +
    // per-type chip + cross-bucket total (only rendered when ≥2 buckets held).
    balanceCardByBucket: (bucketId: string) =>
      `[data-testid="points-balance-card-${bucketId}"]`,
    balanceCardDisabledBadge: (bucketId: string) =>
      `[data-testid="points-balance-card-disabled-${bucketId}"]`,
    balanceTotalByBucket: (bucketId: string) =>
      `[data-testid="points-balance-total-${bucketId}"]`,
    balanceType: (bucketId: string, typeKey: string) =>
      `[data-testid="points-balance-type-${bucketId}-${typeKey}"]`,
    crossBucketTotal: '[data-testid="user-points-cross-bucket-total"]',
    balanceEmpty: '[data-testid="points-balance-empty"]',
    // Transaction bucket dimension (credit-bucket US-CB-006).
    // No header testid exists; only per-row bucket cells — assert on row cells.
    transactionBucketCell: (rowIndex: number) =>
      `[data-testid="transaction-bucket-${rowIndex}"]`,
    filterBucket: '[data-testid="filter-bucket"]',
    // Transaction History
    transactionsSection:
      '[data-testid="transaction-history-table"], [data-testid="no-transactions"]',
    transactionsTable: '[data-testid="transaction-history-table"]',
    transactionRow: (index: number) =>
      `[data-testid="transaction-row-${index}"]`,
    transactionType: (index: number) =>
      `[data-testid="transaction-type-${index}"]`,
    transactionAmount: (index: number) =>
      `[data-testid="transaction-amount-${index}"]`,
    transactionDescription: (index: number) =>
      `[data-testid="transaction-description-${index}"]`,
    transactionTime: (index: number) =>
      `[data-testid="transaction-time-${index}"]`,
    // Transaction Filters
    filterType: '[data-testid="filter-transaction-type"]',
    filterStartTime: '[data-testid="filter-from-date"]',
    filterEndTime: '[data-testid="filter-to-date"]',
    resetFiltersButton: '[data-testid="clear-filters-button"]',
    applyFiltersButton: '[data-testid="apply-filters-button"]',
    exportButton: '[data-testid="export-transactions-button"]',
    // Helper methods for dynamic selectors
    firstTransactionRow: () => '[data-testid^="transaction-row-"]',
  },

  /**
   * Grant Points Dialog Selectors
   * Triggered from: /{realmId}/manage/points/wallets
   *
   * Two-step dialog flow:
   * 1. Form dialog (search user, fill amount/validity/reason)
   * 2. Confirmation dialog (review details, confirm)
   */
  grantPoints: {
    // Wallets page trigger button
    grantPointsButton: '[data-testid="grant-points-button"]',
    // Form dialog
    formDialog: '[data-testid="grant-points-form-dialog"]',
    userSearchInput: '[data-testid="grant-points-user-search-input"] input',
    amountInput: '[data-testid="grant-points-amount-input"]',
    validityDaysInput: '[data-testid="grant-points-validity-days-input"]',
    permanentToggle: '[data-testid="grant-points-permanent-toggle"]',
    reasonInput: '[data-testid="grant-points-reason-input"]',
    cancelButton: '[data-testid="grant-points-cancel-button"]',
    submitButton: '[data-testid="grant-points-submit-button"]',
    // Confirmation dialog
    confirmDialog: '[data-testid="grant-points-confirm-dialog"]',
    confirmButton: '[data-testid="grant-points-confirm-button"]',
    // Error alert
    errorMessage: '[data-testid="grant-points-error-message"]',
    // Target Bucket Select (credit-bucket US-CB / A5: bucketId is required).
    // CONSUMED BY DE-D07 (grant-points-helpers.ts). DE-D01 only declares the
    // selector; it does NOT modify pre-existing grant-points-helpers.ts.
    bucketSelect: '[data-testid="grant-points-bucket-select"]',
  },

  /**
   * Points Configuration Selectors (Admin)
   * Route: /{realmId}/admin/points/default-config
   */
  points: {
    // Realm Configuration
    registrationBonusPointsInput:
      '[data-testid="registration-bonus-points-input"]',
    freePeriodicPointsAmountInput:
      '[data-testid="free-periodic-points-amount-input"]',
    freePeriodicGrantPeriodTypeSelect:
      '[data-testid="grant-period-type-select"]',
    freePeriodicValidityDaysInput:
      '[data-testid="free-periodic-validity-days-input"]',
    saveButton: '[data-testid="save-config-button"]',
    successMessage: '[data-testid="success-message"]',
    errorMessage: '[data-testid="error-message"]',

    // Free User Statistics
    totalFreeUsers: '[data-testid="total-free-users"]',
    activeFreeUsers: '[data-testid="active-free-users"]',
    upgradeRate: '[data-testid="upgrade-rate"]',
    userGrowthChart: '[data-testid="user-growth-chart"]',
    pointsGrantedChart: '[data-testid="points-granted-chart"]',
    upgradeRateChart: '[data-testid="upgrade-rate-chart"]',
    dateRangeFilter: '[data-testid="date-range-filter"]',
    userSearch: '[data-testid="user-search"]',

    // Free User Points
    expiryWarning: '[data-testid="expiry-warning"]',
  },

  /**
   * Registration Page Selectors
   * Route: /{realmId}/auth/register
   */
  registration: {
    emailInput: '[data-testid="register-email-input"]',
    passwordInput: '[data-testid="register-password-input"]',
    confirmPasswordInput: '[data-testid="register-confirm-password-input"]',
    nicknameInput: '[data-testid="register-nickname-input"]',
    registerButton: '[data-testid="register-submit-button"]',
  },

  /**
   * Unified Purchase - Purchase Points Page (User)
   * Route: /{realmId}/user/purchase-points
   */
  purchasePoints: {
    page: '[data-testid="purchase-points-page"]',
    stepIndicator: '[data-testid="purchase-step-indicator"]',
    backButton: '[data-testid="purchase-back-button"]',
    nextButton: '[data-testid="purchase-next-button"]',
    // Steps
    stepPackages: '[data-testid="purchase-step-packages"]', // Mapping card selection step
    stepPayment: '[data-testid="purchase-step-payment"]',
    stepProcessing: '[data-testid="purchase-step-processing"]',
    stepComplete: '[data-testid="purchase-step-complete"]',
  },

  /**
   * Mapping Card Selectors (One-Time Purchase)
   * Displayed on purchase-points page within the mapping-groups grid.
   * Each card uses data-testid="mapping-card-{entitlementKey}".
   */
  mappingCard: {
    grid: '[data-testid="mapping-groups"]',
    card: (entitlementKey: string) => `[data-testid="mapping-card-${entitlementKey}"]`,
    firstCard: () => '[data-testid^="mapping-card-"]',
    noProviderHint: '[data-testid="no-provider-hint"]',
    emptyState: '[data-testid="purchase-empty-state"]',
  },

  /**
   * Unified Purchase - Payment Method Selector (User)
   */
  paymentMethodSelector: {
    container: '[data-testid="payment-method-selector"]',
    button: (platform: string) =>
      `[data-testid="payment-method-button-${platform}"]`,
    select: (platform: string) =>
      `[data-testid="payment-method-select-${platform}"]`,
    selected: (platform: string) =>
      `[data-testid="payment-method-selected-${platform}"]`,
  },

  /**
   * Unified Purchase - Payment Attempt Status (User)
   */
  paymentStatus: {
    container: '[data-testid="payment-status-display"]',
    pending: '[data-testid="payment-status-pending"]',
    requiresAction: '[data-testid="payment-status-requires-action"]',
    succeeded: '[data-testid="payment-status-succeeded"]',
    failed: '[data-testid="payment-status-failed"]',
    cancelled: '[data-testid="payment-status-cancelled"]',
    expired: '[data-testid="payment-status-expired"]',
    countdownTimer: '[data-testid="payment-countdown-timer"]',
    retryButton: '[data-testid="payment-retry-button"]',
    cancelButton: '[data-testid="payment-cancel-button"]',
  },

  /**
   * Payment Provider-Specific UI Selectors
   * Rendered within PaymentAttemptStatus for Pending/RequiresAction states
   */
  paymentProviderUI: {
    // Stripe / Creem redirect flow
    redirectPrompt: '[data-testid="payment-redirect-prompt"]',
    redirectManualLink: '[data-testid="payment-redirect-manual-link"]',
    // Degraded UI (missing payment context)
    contextDegraded: '[data-testid="payment-context-degraded"]',
    // Cancel button (rendered inside provider-specific views)
    cancelButton: '[data-testid="payment-cancel-button"]',
    // Countdown timer (rendered inside provider-specific views)
    countdownTimer: '[data-testid="payment-countdown-timer"]',
  },

  /**
   * Email Configuration Selectors (Settings > Email tab)
   * Route: /{realmId}/manage/settings
   */
  emailConfig: {
    // Tab trigger
    emailTab: '[data-testid="email-tab"]',
    // Status badge
    statusBadge: '[data-testid="email-config-status-badge"]',
    statusError: '[data-testid="email-status-error"]',
    // Provider selection
    providerResend: '[data-testid="email-provider-resend"]',
    providerSmtp: '[data-testid="email-provider-smtp"]',
    // Common fields
    fromAddressInput: '[data-testid="email-from-address-input"]',
    // Resend fields
    resendApiKeyInput: '[data-testid="email-resend-api-key-input"]',
    // SMTP fields
    smtpHostInput: '[data-testid="email-smtp-host-input"]',
    smtpPortInput: '[data-testid="email-smtp-port-input"]',
    smtpEncryptionSelect: '[data-testid="email-smtp-encryption-select"]',
    smtpUsernameInput: '[data-testid="email-smtp-username-input"]',
    smtpPasswordInput: '[data-testid="email-smtp-password-input"]',
    // Test email
    testRecipientInput: '[data-testid="email-test-recipient-input"]',
    testButton: '[data-testid="email-test-button"]',
    testError: '[data-testid="email-test-error"]',
    testSuccess: '[data-testid="email-test-success"]',
    // Save
    saveButton: '[data-testid="email-save-button"]',
    saveError: '[data-testid="email-save-error"]',
  },

  /**
   * Device Verification Page Selectors
   * Route: /{realmId}/device, /{realmId}/device/{userCode}
   */
  deviceVerification: {
    card: '[data-testid="device-verification-card"]',
    title: '[data-testid="device-verification-title"]',
    error: '[data-testid="device-verification-error"]',
    result: '[data-testid="device-verification-result"]',
    codeInput: '[data-testid="device-code-input"]',
    codeSubmit: '[data-testid="device-code-submit"]',
    authorizeButton: '[data-testid="device-authorize-button"]',
    denyButton: '[data-testid="device-deny-button"]',
  },

  /**
   * Credit Bucket Directory (Admin)
   *
   * Route: /{realmId}/manage/billing/credit-buckets
   *
   * LOUD NOTE — i18n-dependent sidebar entry:
   * The sidebar menu item testid `sidebar-menu-credit-buckets` is derived in
   * `components/admin/sidebar.tsx` from the localized label
   * `m['nav.credit_buckets']().toLowerCase().replace(/\s+/g,'-')`. The testid
   * therefore differs per locale. Demo tests MUST navigate by route
   * (`/{realmId}/manage/billing/credit-buckets`), NOT by clicking the
   * locale-derived sidebar testid. See `.ai/design/credit-bucket.md` §4.4.2/§7
   * and `demo/dev/dev.md` loud notes.
   *
   * User stories: US-CB-001 (admin CRUD), US-CB-002 (coverage set),
   * US-CB-003 (mapping→bucket assignment).
   *
   * Verified testids: frontend/src/components/billing/credit-bucket/*.tsx +
   * frontend/src/routes/$realmId/manage/billing/credit-buckets.tsx.
   */
  creditBucket: {
    // Directory page container + toolbar
    directoryPage: '[data-testid="credit-buckets-directory-page"]',
    searchInput: '[data-testid="credit-bucket-search-input"]',
    newButton: '[data-testid="credit-bucket-new-button"]',
    emptyNewButton: '[data-testid="credit-bucket-empty-new-button"]',
    // List item + per-bucket badges
    listItem: (bucketId: string) =>
      `[data-testid="credit-bucket-list-item-${bucketId}"]`,
    listItemRegistrationBadge: (bucketId: string) =>
      `[data-testid="credit-bucket-list-item-${bucketId}-registration-badge"]`,
    listItemDisabledBadge: (bucketId: string) =>
      `[data-testid="credit-bucket-list-item-${bucketId}-disabled-badge"]`,
    // Empty / no-selection states
    emptyState: '[data-testid="credit-buckets-empty-state"]',
    noSelection: '[data-testid="credit-buckets-no-selection"]',
    // Editor (used for both create and edit)
    editor: '[data-testid="credit-bucket-editor"]',
    editorName: '[data-testid="credit-bucket-editor-name"]',
    editorBucketKey: '[data-testid="credit-bucket-editor-bucket-key"]',
    editorDescription: '[data-testid="credit-bucket-editor-description"]',
    editorEnabled: '[data-testid="credit-bucket-editor-enabled"]',
    editorRegistration: '[data-testid="credit-bucket-editor-registration"]',
    editorRegistrationConflict:
      '[data-testid="credit-bucket-editor-registration-conflict"]',
    editorSubmit: '[data-testid="credit-bucket-editor-submit"]',
    // Delete dialog (destructive op)
    deleteButton: '[data-testid="credit-bucket-delete-button"]',
    deleteConfirmDialog: '[data-testid="delete-bucket-confirm-dialog"]',
    deleteErrorMessage: '[data-testid="delete-bucket-error-message"]',
    deleteConfirmButton: '[data-testid="delete-bucket-confirm-button"]',
    deleteCancelButton: '[data-testid="delete-bucket-cancel-button"]',
    // Overview (matrix audit) page — Route: /{realmId}/manage/billing/credit-buckets/overview
    overviewPage: '[data-testid="credit-bucket-overview-page"]',
    overviewToolbar: '[data-testid="credit-bucket-overview-toolbar"]',
    overviewTable: '[data-testid="credit-bucket-overview-table"]',
    overviewGrandTotal: '[data-testid="credit-bucket-overview-grandtotal"]',
    overviewGrandTotalByKey: (key: string) =>
      `[data-testid="credit-bucket-overview-grandtotal-${key}"]`,
    overviewColTotalByKey: (key: string) =>
      `[data-testid="credit-bucket-overview-col-total-${key}"]`,
    overviewRow: (bucketId: string) =>
      `[data-testid="credit-bucket-overview-row-${bucketId}"]`,
    overviewCell: (bucketId: string, key: string) =>
      `[data-testid="credit-bucket-overview-cell-${bucketId}-${key}"]`,
    overviewDisabled: (bucketId: string) =>
      `[data-testid="credit-bucket-overview-disabled-${bucketId}"]`,
    overviewRegistration: (bucketId: string) =>
      `[data-testid="credit-bucket-overview-registration-${bucketId}"]`,
    overviewDetail: (bucketId: string) =>
      `[data-testid="credit-bucket-overview-detail-${bucketId}"]`,
    overviewEmptyState: '[data-testid="credit-bucket-overview-empty-state"]',
    overviewEmptyCta: '[data-testid="credit-bucket-overview-empty-cta"]',
    // Coverage set multiselect (binds client apps to a bucket).
    // Prefix is `bucket-coverage-multiselect` (frontend credit-bucket-coverage-multiselect.tsx).
    coverageMultiselect: '[data-testid="bucket-coverage-multiselect"]',
    coverageMultiselectSearch: '[data-testid="bucket-coverage-multiselect-search"]',
    coverageMultiselectError: '[data-testid="bucket-coverage-multiselect-error"]',
    coverageMultiselectItem: (clientAppId: string) =>
      `[data-testid="bucket-coverage-multiselect-item-${clientAppId}"]`,
    // Mappings multiselect (assigns entitlement mappings to a bucket).
    // Prefix is `bucket-mappings-multiselect` (frontend credit-bucket-mappings-multiselect.tsx).
    mappingsMultiselect: '[data-testid="bucket-mappings-multiselect"]',
    mappingsMultiselectSearch: '[data-testid="bucket-mappings-multiselect-search"]',
    mappingsMultiselectError: '[data-testid="bucket-mappings-multiselect-error"]',
    mappingsMultiselectItem: (mappingId: string) =>
      `[data-testid="bucket-mappings-multiselect-item-${mappingId}"]`,
  },

  /**
   * Unified Purchase - Purchase History (User)
   * Route: /{realmId}/user/subscription-history
   */
  purchaseHistory: {
    page: '[data-testid="purchase-records-page"]',
    list: '[data-testid="purchase-history-list"]',
    loading: '[data-testid="purchase-history-loading"]',
    empty: '[data-testid="purchase-history-empty"]',
    error: '[data-testid="purchase-history-error"]',
    item: (id: string) => `[data-testid="purchase-history-item-${id}"]`,
    detailsButton: (id: string) =>
      `[data-testid="purchase-history-details-button-${id}"]`,
    // Filters
    filterProvider: '[data-testid="filter-provider"]',
    filterStatus: '[data-testid="filter-status"]',
    filterFromDate: '[data-testid="filter-from-date"]',
    filterToDate: '[data-testid="filter-to-date"]',
    resetFiltersButton: '[data-testid="reset-filters-button"]',
    applyFiltersButton: '[data-testid="apply-filters-button"]',
  },
};

/**
 * Selector helper for multiple fallback selectors
 *
 * @example
 * const button = page.locator(getSelector(SELECTORS.common.dialogSubmitButton))
 */
export function getSelector(selector: string | string[]): string {
  if (Array.isArray(selector)) {
    return selector.join(", ");
  }
  return selector;
}
