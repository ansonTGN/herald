# Client App Wizard

A comprehensive multi-step wizard for creating and editing OAuth 2.0 client applications in the Herald system.

## Overview

The Client App Wizard provides a guided, step-by-step interface for configuring OAuth 2.0 client applications with real-time validation, auto-save functionality, and a polished user experience.

## Features

### Core Functionality

- **4-Step Wizard Flow**: Linear progression through configuration steps
- **Create & Edit Modes**: Support for both new and existing client apps
- **Real-time Validation**: Instant feedback on form inputs
- **Auto-Save System**: Automatic draft saving with restore capability
- **Responsive Design**: Works seamlessly on desktop and mobile devices

### User Experience

- **Smooth Animations**: GPU-accelerated transitions for 60fps performance
- **Keyboard Navigation**: Full keyboard support for accessibility
- **Progress Indicator**: Visual progress tracking with step labels
- **Contextual Help**: Inline help text and explanations throughout
- **Error Handling**: Clear error messages and validation feedback

### Performance

- **Code Splitting**: React.lazy for optimal bundle size
- **GPU Acceleration**: Hardware-accelerated animations
- **Optimized Re-renders**: Efficient React rendering patterns
- **Lazy Loading**: Components loaded on-demand

### Accessibility

- **WCAG AA Compliant**: Meets accessibility standards
- **ARIA Labels**: Proper screen reader support
- **Keyboard Navigation**: Full keyboard control
- **Focus Management**: Proper focus handling in modals
- **Color Contrast**: High contrast for readability

## Wizard Steps

### Step 1: Basic Information

Configure the fundamental properties of the client application.

**Fields:**

- **App Name** (required): Display name for the application (1-100 characters)
- **Description** (optional): Application purpose description (max 500 characters)
- **App Type** (required): Application type
  - `WEB`: Web applications running in a browser
  - `SERVICE`: Service-to-service applications
  - `MOBILE`: Mobile applications
  - `NATIVE`: Desktop/native applications
- **Client Type** (required): OAuth 2.0 client type
  - `CONFIDENTIAL`: Clients that can securely store credentials
  - `PUBLIC`: Clients that cannot securely store credentials

### Step 2: Redirect URIs

Configure OAuth 2.0 redirect endpoints and CORS settings.

**Fields:**

- **Valid Redirect URIs** (required): OAuth 2.0 callback URLs
- **Valid Post Logout URIs** (optional): Post-logout redirect URLs
- **Web Origins** (optional): CORS allowed origins

**Advanced Settings:**

- **Advanced CORS Settings**: Collapsible section for fine-grained CORS control

**Validation:**

- URLs must be valid and start with `https://` (or `http://` for development)
- Duplicate URIs are automatically rejected
- Real-time format validation with visual feedback

### Step 3: Security Settings

Configure session management and security policies.

**Fields:**

- **Session TTL** (required): Session duration in seconds (60-86400)
  - Presets: 15 min, 1 hour, 8 hours, 24 hours
  - Custom value input available
- **Session Renewal TTL** (optional): Sliding window for silent renewal
  - Must be greater than Session TTL
  - Creates "sliding session" experience

**Advanced Settings:**

- **Advanced Security Options**: Future OAuth 2.0 security configurations
  - (Currently placeholder for future backend API support)

### Step 4: Review & Create/Save

Review all configurations before submission.

**Features:**

- **Summary Display**: Complete overview of all settings
- **Edit Capability**: Click any section to return and modify
- **Final Validation**: Last check before submission
- **Submission**: Create new app or save changes

## Auto-Save System

### How It Works

The auto-save system automatically saves form data to localStorage every 30 seconds during create mode, preventing data loss from accidental navigation or browser closure.

### Features

- **Automatic Saving**: Saves every 30 seconds
- **Draft Restoration**: Prompts to restore draft on component mount
- **Version Validation**: Ensures draft schema compatibility
- **Clear on Complete**: Removes draft after successful submission

### Storage Key Format

```
client-app-draft-{realmId}-{mode}-{clientAppId}
```

### Draft Data Structure

```typescript
{
  data: Partial<WizardFormData>,
  timestamp: number,
  version: string
}
```

## Component Architecture

### Main Components

#### ClientAppWizard

Main wizard component that orchestrates the multi-step flow.

**Location:** `src/components/client-apps/client-app-wizard.tsx`

**Props:**

- `mode`: `'create' | 'edit'`
- `realmId`: `string`
- `initialData`: `ClientAppItem` (optional)

#### Wizard Steps

Individual step components for each configuration phase.

**Location:** `src/components/client-apps/wizard-steps/`

- `Step1Basic`: Basic information form
- `Step2Redirects`: Redirect URI configuration
- `Step3Security`: Security settings
- `Step4Review`: Summary and submission

#### Supporting Components

- `ProgressIndicator`: Visual progress tracking
- `RedirectUrisInput`: Dynamic URI list input
- `AdvancedSettingsCollapsible`: Expandable advanced options
- `DraftRestoreDialog`: Draft restoration prompt

### Data Flow

```
User Input → Form Validation → State Update → Auto-Save (if applicable)
                                                        ↓
                                         Step Transition → Progress Update
                                                        ↓
                                         Final Submission → API Call → Navigation
```

## Performance Optimizations

### Code Splitting

Heavy components are lazy-loaded for optimal bundle size:

```typescript
const ClientAppWizard = lazy(() => import('@/components/client-apps/client-app-wizard'))
```

### GPU Acceleration

Animations use hardware acceleration for smooth 60fps performance:

```css
.animate-step-enter {
  will-change: transform, opacity;
  transform: translateZ(0);
  backface-visibility: hidden;
}
```

### Optimized Imports

Tree-shakeable imports minimize bundle size:

```typescript
import { Step1Basic } from './wizard-steps'
```

## Accessibility Features

### Keyboard Navigation

- **Tab**: Navigate between form fields
- **Enter**: Submit forms or add items
- **Escape**: Close modals/dialogs
- **Arrow Keys**: Navigate within radio groups and lists

### ARIA Attributes

- Proper `role` attributes for semantic meaning
- `aria-label` for button and input descriptions
- `aria-describedby` for help text associations
- `aria-expanded` for collapsible sections
- `aria-live` for dynamic content updates

### Screen Reader Support

- Clear labels for all form inputs
- Error announcements with `role="alert"`
- Progress announcements with `aria-live`
- Descriptive link and button text

### Focus Management

- Automatic focus on first input in modals
- Focus trapping in drawers
- Visible focus indicators
- Logical tab order

## Usage Examples

### Create Mode

```tsx
import { ClientAppWizard } from '@/components/client-apps/client-app-wizard'

function NewClientAppPage() {
  const { realmId } = useParams()

  return (
    <div className="container max-w-3xl mx-auto py-12 px-6">
      <ClientAppWizard mode="create" realmId={realmId} />
    </div>
  )
}
```

### Edit Mode

```tsx
import { ClientAppWizard } from '@/components/client-apps/client-app-wizard'
import { useSuspenseQuery } from '@tanstack/react-query'

function EditClientAppPage() {
  const { realmId, clientAppId } = useParams()
  const { data: clientApp } = useSuspenseQuery(clientAppQueryOptions(realmId, clientAppId))

  return (
    <div className="container max-w-3xl mx-auto py-12 px-6">
      <ClientAppWizard mode="edit" realmId={realmId} initialData={clientApp} />
    </div>
  )
}
```

### With Lazy Loading

```tsx
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const ClientAppWizard = lazy(() =>
  import('@/components/client-apps/client-app-wizard').then((m) => ({
    default: m.ClientAppWizard,
  }))
)

function NewClientAppPage() {
  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <ClientAppWizard mode="create" realmId={realmId} />
    </Suspense>
  )
}
```

## Validation

### Client-Side Validation

All forms use Zod schemas for runtime validation:

```typescript
import { z } from 'zod'

export const step1Schema = z.object({
  name: z.string().min(1).max(100),
  description: z.string().max(500).optional(),
  appType: z.enum(['WEB', 'SERVICE', 'MOBILE', 'NATIVE']),
  clientType: z.enum(['CONFIDENTIAL', 'PUBLIC']),
})
```

### Server-Side Validation

The backend API performs additional validation on submission.

### Error Handling

- **Field Errors**: Displayed below relevant inputs
- **Form Errors**: Shown as toast notifications
- **Network Errors**: User-friendly error messages

## Styling

### CSS Custom Properties

The wizard uses Tailwind CSS with custom properties for theming:

```css
:root {
  --primary: oklch(0.21 0.006 285.885);
  --primary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.967 0.001 286.375);
  --muted-foreground: oklch(0.552 0.016 285.938);
  /* ... more colors */
}
```

### Animation Classes

Custom animation classes for smooth transitions:

```css
.animate-step-enter {
  /* Step entry animation */
}
.animate-step-exit {
  /* Step exit animation */
}
.animate-slide-in {
  /* Slide-in effect */
}
.animate-slide-down {
  /* Slide-down effect */
}
```

## Testing

### Unit Tests

Located in `src/components/client-apps/*.test.tsx`

Run tests:

```bash
npm run test:run -- client-app-wizard
```

### E2E Tests

Test IDs are provided for automated testing:

```typescript
data-testid="client-app-wizard"
data-testid="wizard-step-basic"
data-testid="client-app-name-input"
data-testid="next-button"
```

## Browser Support

- Chrome/Edge: Last 2 versions
- Firefox: Last 2 versions
- Safari: Last 2 versions
- Mobile browsers: iOS Safari, Chrome Mobile

## Performance Metrics

### Target Metrics

- **LCP** (Largest Contentful Paint): < 2.5s
- **FID** (First Input Delay): < 100ms
- **CLS** (Cumulative Layout Shift): < 0.1

### Bundle Size

- Main wizard component: ~15KB (gzipped)
- All wizard steps: ~25KB (gzipped)
- Total with dependencies: ~150KB (gzipped)

## Future Enhancements

### Planned Features

- [ ] Advanced OAuth 2.0 grant type selection
- [ ] Custom token expiration settings
- [ ] Client authentication method configuration
- [ ] Additional security options
- [ ] Import/export client app configurations
- [ ] Bulk operations for client app management

### Backend Integration

- [ ] Advanced security options API support
- [ ] Custom grant type configuration
- [ ] Client authentication method selection
- [ ] Fine-grained permission settings

## Troubleshooting

### Common Issues

**Draft not restoring:**

- Check localStorage availability
- Verify draft key format
- Ensure version compatibility

**Animations not smooth:**

- Check GPU acceleration support
- Verify `prefers-reduced-motion` setting
- Test in different browsers

**Form validation failing:**

- Check Zod schema definitions
- Verify API request/response types
- Review console for validation errors

## Contributing

When modifying the wizard:

1. **Maintain Accessibility**: Test with screen readers and keyboard
2. **Preserve Performance**: Monitor bundle size and render performance
3. **Update Tests**: Add/modify unit tests for new features
4. **Document Changes**: Update this README with new functionality
5. **Test Thoroughly**: Test in both create and edit modes

## License

Part of the Herald project.

## Related Components

- [TotpSetupPage](../auth/totp-setup-page.tsx) - TOTP setup page
- [ProgressIndicator](../ui/progress-indicator.tsx) - Progress tracking component
- [RedirectUrisInput](./redirect-uris-input.tsx) - Dynamic URI input component
- [DraftRestoreDialog](./draft-restore-dialog.tsx) - Draft restoration dialog

## See Also

- [TanStack Router](https://tanstack.com/router) - Routing library
- [TanStack Form](https://tanstack.com/form) - Form management
- [Zod](https://zod.dev) - Schema validation
- [Tailwind CSS](https://tailwindcss.com) - Styling framework
