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
    menuAuditLog: '[data-testid="sidebar-menu-audit-log"]',
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
    addButton: '[data-testid="add-user-button"], [data-testid="create-user-button"]',
    searchInput: '[data-testid="users-search-input"]',
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
    createDescriptionInput: '[data-testid="permission-create-description-input"]',
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
    addButton: '[data-testid="add-realm-button"], [data-testid="create-realm-button"]',
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
    totpSavedBackupCodesCheckbox: '[data-testid="totp-saved-backup-codes-checkbox"]',
    totpSavedBackupCodesLabel: '[data-testid="totp-saved-backup-codes-label"]',
    // TOTP Disable Form (Dialog)
    totpDisablePasswordInput: '[data-testid="totp-disable-password-input"]',
    totpDisableCancelButton: '[data-testid="totp-disable-cancel-button"]',
    totpDisableSubmitButton: '[data-testid="totp-disable-submit-button"]',
    // TOTP Regenerate Form (Dialog)
    totpRegeneratePasswordInput: '[data-testid="totp-regenerate-password-input"]',
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
    redirectUrisInput: '[data-testid="redirect-uris-input"]',
    // Security tab
    enabledSwitch: '[data-testid="client-app-enabled-switch"]',
    sessionTtlInput: '[data-testid="session-ttl-input"]',
    sessionTtlPreset: (label: string) => `[data-testid="session-ttl-preset-${label}"]`,
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
    toastMessage: '[data-testid="toast-message"], [data-sonner-toast] [data-description]',
    successMessage: '[data-testid="success-message"], [data-sonner-toast].success',
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
    viewDetailsButton: (index: number) => `[data-testid="view-details-${index}"]`,
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
    viewEventDetailsButton: (index: number) => `[data-testid="view-event-details-${index}"]`,
    // Event details dialog
    eventDetailDialog: '[data-testid="event-detail-dialog"]',
    // Event badges (parameterized by event type)
    eventBadge: (
      type:
        | 'created'
        | 'upgraded'
        | 'downgraded'
        | 'canceled'
        | 'renewed'
        | 'reactivated'
        | 'expired'
    ) => `[data-testid="event-badge-${type}"]`,
  },

  /**
   * Billing Page Selectors
   * Route: /{realmId}/manage/billing
   */
  billing: {
    page: '[data-testid="billing-page"]',
    // Plan table
    table: '[data-testid="plans-table"]',
    addPlanButton: '[data-testid="add-plan-button"]',
    // Plan form page (navigated to from billing page)
    planFormPage: '[data-testid="plan-form-page"]',
    planFormBackButton: '[data-testid="plan-form-back-button"]',
    planFormTitle: '[data-testid="plan-form-title"]',
    planNameInput: '[data-testid="plan-name-input"]',
    planTitleInput: '[data-testid="plan-title-input"]',
    planDescriptionInput: '[data-testid="plan-description-input"]',
    planTypeSelectTrigger: '[data-testid="plan-type-select-trigger"]',
    planTypeMonthly: '[data-testid="plan-type-monthly"]',
    planTypeYearly: '[data-testid="plan-type-yearly"]',
    planPriceInput: '[data-testid="plan-price-input"]',
    planCurrencySelectTrigger: '[data-testid="plan-currency-select-trigger"]',
    planCurrencyUsd: '[data-testid="plan-currency-usd"]',
    planTrialDaysInput: '[data-testid="plan-trial-days-input"]',
    planCheckoutUrlInput: '[data-testid="plan-checkout-url-input"]',
    planProductSelectTrigger: '[data-testid="plan-product-select-trigger"]',
    planFormSubmitButton: '[data-testid="plan-form-submit-button"]',
    planFormCancelButton: '[data-testid="plan-form-cancel-button"]',
    // Provider mapping page
    providerMappingPage: '[data-testid="plan-providers-page"]',
    providerMappingList: '[data-testid="provider-mapping-list"]',
    addProviderMappingButton: '[data-testid="add-provider-mapping-button"]',
    providerMappingFormDialog: '[data-testid="provider-mapping-form-dialog"]',
    providerMappingProviderSelectTrigger:
      '[data-testid="provider-mapping-provider-select-trigger"]',
    providerMappingExternalProductIdInput: '[data-testid="provider-mapping-product-id-input"]',
    providerMappingExternalPriceIdInput: '[data-testid="provider-mapping-price-id-input"]',
    providerMappingSubmitButton: '[data-testid="provider-mapping-submit-button"]',
    providerMappingCancelButton: '[data-testid="provider-mapping-cancel-button"]',
    providerMappingBackButton: '[data-testid="back-to-billing-button"]',
    // Dynamic provider mapping buttons
    editMappingButton: (id: string) => `[data-testid="edit-mapping-button-${id}"]`,
    toggleMappingButton: (id: string) => `[data-testid="toggle-mapping-button-${id}"]`,
    deleteMappingButton: (id: string) => `[data-testid="delete-mapping-button-${id}"]`,
    // Delete confirmation dialog
    confirmDeleteButton: '[data-testid="confirm-delete-button"]',
  },

  /**
   * Points Management Page Selectors (Admin)
   * Route: /{realmId}/manage/points
   */
  pointsAdmin: {
    // Page containers
    accountsPage: '[data-testid="points-wallets-page"]',
    configsPage: '[data-testid="points-configs-page"]',
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
    transactionRow: (index: number) => `[data-testid="transaction-row-${index}"]`,
    transactionType: (index: number) => `[data-testid="transaction-type-${index}"]`,
    transactionAmount: (index: number) => `[data-testid="transaction-amount-${index}"]`,
    transactionBalance: (index: number) => `[data-testid="transaction-balance-${index}"]`,
    transactionDescription: (index: number) => `[data-testid="transaction-description-${index}"]`,
    transactionTime: (index: number) => `[data-testid="transaction-time-${index}"]`,
    // Transaction Filters
    transactionFilters: '[data-testid="transaction-filters"]',
    filterType: '[data-testid="filter-transaction-type"]',
    filterStartTime: '[data-testid="filter-from-date"]',
    filterEndTime: '[data-testid="filter-to-date"]',
    filterClientApp: '[data-testid="filter-client-app"]',
    resetFiltersButton: '[data-testid="clear-filters-button"]',
    applyFiltersButton: '[data-testid="apply-filters-button"]',
    // Plan Configs Section
    planConfigsSection: '[data-testid="points-configs-page"]',
    planConfigsTable: '[data-testid="points-configs-page"]',
    planConfigRow: (configId: string) => `[data-testid="config-card-${configId}"]`,
    createPlanConfigButton: '[data-testid="create-config-button"]',
    editPlanConfigButton: (configId: string) => `[data-testid="edit-config-${configId}"]`,
    deletePlanConfigButton: (configId: string) => `[data-testid="delete-config-${configId}"]`,
    // Plan Config Dialog
    planConfigDialog: '[role="dialog"] [data-testid="points-plan-config-form"]',
    planConfigPlanId: '[data-testid="plan-select"]',
    planConfigPointsOnSubscribe: '[data-testid="points-per-period"]',
    planConfigRenewalEnabled: '[data-testid="grant-on-subscribe"]',
    planConfigRenewalPeriodType: '[data-testid="grant-period-type"]',
    planConfigValidityDays: '[data-testid="validity-days"]',
    planConfigMaxAccumulation: '[data-testid="max-periods"]',
    planConfigSubmitButton: '[data-testid="submit-button"]',
    planConfigCancelButton: '[data-testid="cancel-button"]',
    // Helper methods for dynamic selectors
    firstEditPlanConfigButton: () => '[data-testid^="edit-config-"]',
    firstDeletePlanConfigButton: () => '[data-testid^="delete-config-"]',
    firstViewGuideButton: () => '[data-testid^="points-view-guide-"]',
    firstShareGuideButton: () => '[data-testid^="points-share-guide-"]',
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
    // Transaction History
    transactionsSection:
      '[data-testid="transaction-history-table"], [data-testid="no-transactions"]',
    transactionsTable: '[data-testid="transaction-history-table"]',
    transactionRow: (index: number) => `[data-testid="transaction-row-${index}"]`,
    transactionType: (index: number) => `[data-testid="transaction-type-${index}"]`,
    transactionAmount: (index: number) => `[data-testid="transaction-amount-${index}"]`,
    transactionDescription: (index: number) => `[data-testid="transaction-description-${index}"]`,
    transactionTime: (index: number) => `[data-testid="transaction-time-${index}"]`,
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
   * Points Configuration Selectors (Admin)
   * Route: /{realmId}/admin/points/realm-config
   */
  points: {
    // Realm Configuration
    registrationBonusPointsInput: '[data-testid="registration-bonus-points-input"]',
    freePeriodicPointsAmountInput: '[data-testid="free-periodic-points-amount-input"]',
    freePeriodicGrantPeriodTypeSelect: '[data-testid="grant-period-type-select"]',
    freePeriodicValidityDaysInput: '[data-testid="free-periodic-validity-days-input"]',
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
   * WeChat Pay Configuration Selectors
   * Route: /{realmId}/billing/payment-providers
   */
  wechatPay: {
    // Configuration card
    configCard: '[data-testid="wechat-config-detail"]',
    appIdDisplay: '[data-testid="app-id-display"]',
    merchantIdDisplay: '[data-testid="merchant-id-display"]',
    serialNoDisplay: '[data-testid="serial-no-display"]',
    v3KeyDisplay: '[data-testid="v3-key-display"]',
    privateKeyDisplay: '[data-testid="private-key-display"]',
    notifyUrlDisplay: '[data-testid="notify-url-display"]',
    showSecretsButton: '[data-testid="wechat-config-detail"] [data-testid="show-secrets-button"]',
    hideSecretsButton: '[data-testid="hide-secrets-button"]',
    editConfigButton: '[data-testid="edit-wechat-config-button"]',
    deleteConfigButton: '[data-testid="delete-wechat-config-button"]',

    // Configuration form
    configFormDialog: '[data-testid="wechat-config-form-dialog"]',
    configForm: '[data-testid="wechat-config-form"]',
    appIdInput: '[data-testid="app-id-input"]',
    merchantIdInput: '[data-testid="merchant-id-input"]',
    serialNoInput: '[data-testid="serial-no-input"]',
    v3KeyInput: '[data-testid="v3-key-input"]',
    notifyUrlInput: '[data-testid="notify-url-input"]',
    privateKeyInput: '[data-testid="private-key-input"]',
    configSubmitButton: '[data-testid="wechat-config-submit-button"]',
    configCancelButton: '[data-testid="wechat-config-cancel-button"]',

    // Payment providers page
    addWechatButton: '[data-testid="add-wechat-button"]',
    editWechatButton: '[data-testid="edit-wechat-button"]',
    deleteWechatButton: '[data-testid="delete-wechat-button"]',

    // QR Code Payment
    qrPaymentContainer: '[data-testid="wechat-qr-payment"]',
    qrCodeContainer: '[data-testid="wechat-qr-code-container"]',
    qrCode: '[data-testid="wechat-qr-code"]',
    qrCountdownDisplay: '[data-testid="qr-countdown-display"]',
    creatingOrderState: '[data-testid="creating-order-state"]',
    expiredState: '[data-testid="expired-state"]',
    errorState: '[data-testid="error-state"]',
    cancelPaymentButton: '[data-testid="cancel-payment-button"]',
    providerDisabledTooltip: '[data-testid="provider-disabled-tooltip"]',
  },

  /**
   * Unified Purchase - Points Packages (Admin)
   * Route: /{realmId}/manage/points-packages
   */
  pointsPackages: {
    page: '[data-testid="points-packages-page"]',
    table: '[data-testid="points-packages-table"]',
    addButton: '[data-testid="add-points-package-button"]',
    loadingSkeleton: '[data-testid="points-packages-loading-skeleton"]',
    emptyState: '[data-testid="points-packages-empty-state"]',
    error: '[data-testid="points-packages-error"]',
    // Package list items
    editButton: (id: string) => `[data-testid="points-package-edit-button-${id}"]`,
    configureButton: (id: string) => `[data-testid="points-package-configure-button-${id}"]`,
    deleteButton: (id: string) => `[data-testid="points-package-delete-button-${id}"]`,
  },

  /**
   * Unified Purchase - Points Package Form Dialog
   */
  pointsPackageForm: {
    dialog: '[data-testid="points-package-form-dialog"]',
    nameInput: '[data-testid="points-package-name-input"]',
    titleInput: '[data-testid="points-package-title-input"]',
    descriptionInput: '[data-testid="points-package-description-input"]',
    pointsInput: '[data-testid="points-package-points-input"]',
    priceInput: '[data-testid="points-package-price-input"]',
    currencySelect: '[data-testid="points-package-currency-select"]',
    sortOrderInput: '[data-testid="points-package-sort-order-input"]',
    enabledSwitch: '[data-testid="points-package-enabled-switch"]',
    cancelButton: '[data-testid="points-package-cancel-button"]',
    submitButton: '[data-testid="points-package-submit-button"]',
  },

  /**
   * Unified Purchase - Payment Provider Config Form
   */
  paymentProviderConfig: {
    page: '[data-testid="points-package-providers-page"]',
    providerSelect: '[data-testid="provider-mapping-provider-select-trigger"]',
    // WeChat Pay fields
    wechatAppId: '[data-testid="wechat-app-id"]',
    wechatMerchantId: '[data-testid="wechat-merchant-id"]',
    wechatSerialNo: '[data-testid="wechat-serial-no"]',
    wechatV3Key: '[data-testid="wechat-v3-key"]',
    // Stripe fields
    stripeProductId: '[data-testid="stripe-product-id"]',
    stripePriceId: '[data-testid="stripe-price-id"]',
    // Provider-specific fields
    externalIdInput: () => '[data-testid="provider-mapping-product-id-input"]',
    enabledSwitch: () => '[data-testid="provider-mapping-enabled-switch"]',
    addButton: '[data-testid="provider-mapping-submit-button"]',
    // Common fields
    cancelButton: '[data-testid="provider-config-cancel-button"]',
    submitButton: '[data-testid="provider-config-submit-button"]',
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
    stepPackages: '[data-testid="purchase-step-packages"]',
    stepPayment: '[data-testid="purchase-step-payment"]',
    stepProcessing: '[data-testid="purchase-step-processing"]',
    stepComplete: '[data-testid="purchase-step-complete"]',
  },

  /**
   * Unified Purchase - Points Package Selector (User)
   */
  packageSelector: {
    container: '[data-testid="points-packages-selector"]',
    card: (id: string) => `[data-testid="points-package-card-${id}"]`,
    selectButton: (id: string) => `[data-testid="points-package-select-button-${id}"]`,
    selected: (id: string) => `[data-testid="points-package-selected-${id}"]`,
    bestValueBadge: '[data-testid="points-package-best-value-badge"]',
  },

  /**
   * Unified Purchase - Payment Method Selector (User)
   */
  paymentMethodSelector: {
    container: '[data-testid="payment-method-selector"]',
    button: (platform: string) => `[data-testid="payment-method-button-${platform}"]`,
    select: (platform: string) => `[data-testid="payment-method-select-${platform}"]`,
    selected: (platform: string) => `[data-testid="payment-method-selected-${platform}"]`,
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
    // WeChat QR code flow
    wechatQrSection: '[data-testid="payment-wechat-qr-section"]',
    wechatQrCode: '[data-testid="wechat-qr-code"]',
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
   * Unified Purchase - Purchase History (User)
   * Route: /{realmId}/user/purchase-history
   */
  purchaseHistory: {
    page: '[data-testid="purchase-history-page"]',
    list: '[data-testid="purchase-history-list"]',
    loading: '[data-testid="purchase-history-loading"]',
    empty: '[data-testid="purchase-history-empty"]',
    error: '[data-testid="purchase-history-error"]',
    item: (id: string) => `[data-testid="purchase-history-item-${id}"]`,
    detailsButton: (id: string) => `[data-testid="purchase-history-details-button-${id}"]`,
    // Filters
    filterProvider: '[data-testid="filter-provider"]',
    filterStatus: '[data-testid="filter-status"]',
    filterFromDate: '[data-testid="filter-from-date"]',
    filterToDate: '[data-testid="filter-to-date"]',
    resetFiltersButton: '[data-testid="reset-filters-button"]',
    applyFiltersButton: '[data-testid="apply-filters-button"]',
  },
}

/**
 * Selector helper for multiple fallback selectors
 *
 * @example
 * const button = page.locator(getSelector(SELECTORS.common.dialogSubmitButton))
 */
export function getSelector(selector: string | string[]): string {
  if (Array.isArray(selector)) {
    return selector.join(', ')
  }
  return selector
}
