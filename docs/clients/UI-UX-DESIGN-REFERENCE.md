# WRAITH Client Applications - UI/UX Design Reference

**Version:** 1.1.0
**Last Updated:** 2026-01-21
**Applies To:** All WRAITH desktop and mobile client applications

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
   - [Standardization Status](#15-standardization-status)
2. [Design Strategy](#2-design-strategy)
3. [Visual Design System](#3-visual-design-system)
   - [Text Color Hierarchy](#314-text-color-hierarchy-standardized)
   - [Border Color Hierarchy](#315-border-color-hierarchy-standardized)
4. [Component Library](#4-component-library)
   - [Icon Button Accessibility](#414-icon-button-accessibility-requirements)
5. [Layout Principles](#5-layout-principles)
6. [Interaction Patterns](#6-interaction-patterns)
7. [Application-Specific Guidelines](#7-application-specific-guidelines)
8. [Technical Implementation](#8-technical-implementation)
   - [Tailwind v4 CSS Architecture](#812-tailwind-v4-css-architecture)
   - [Complete CSS Template](#813-complete-css-template)
9. [Accessibility Standards](#9-accessibility-standards)
10. [Future Enhancements](#10-future-enhancements)
11. [Migration Guide](#11-migration-guide)
    - [Migrating from Tailwind v3 to v4](#111-migrating-from-tailwind-v3-to-v4)
    - [Standardization Checklist](#112-standardization-checklist-for-new-clients)
    - [Common Issues and Solutions](#113-common-issues-and-solutions)

---

## 1. Executive Summary

### 1.1 Purpose

This document establishes the definitive UI/UX design reference for all WRAITH Protocol client applications. It provides:

- Consistent visual language across all WRAITH applications
- Reusable component patterns and implementation guidelines
- Accessibility standards ensuring WCAG 2.1 AA compliance
- Technical specifications for Tauri 2.0 + React + TypeScript + Tailwind CSS

### 1.2 Scope

**Current Applications:**
| Application | Status | Primary Function |
|-------------|--------|------------------|
| WRAITH-Transfer | Production | Secure file transfer |
| WRAITH-Chat | Production | E2EE messaging, voice/video calls |
| WRAITH-Sync | Production | File synchronization |
| WRAITH-Android | Production | Mobile messaging (Android) |
| WRAITH-iOS | Production | Mobile messaging (iOS) |

**Planned Applications:**
| Application | Status | Primary Function |
|-------------|--------|------------------|
| WRAITH-Share | Planned | Distributed file sharing |
| WRAITH-Stream | Planned | Secure media streaming |
| WRAITH-Mesh | Planned | IoT mesh networking |
| WRAITH-Publish | Planned | Censorship-resistant publishing |
| WRAITH-Vault | Planned | Distributed secret storage |

### 1.3 Design Philosophy

WRAITH applications embody three core principles:

1. **Security-First Visual Language**: Every design decision reinforces the secure, privacy-focused nature of the protocol
2. **Functional Minimalism**: Clean interfaces that prioritize functionality without unnecessary decoration
3. **Progressive Disclosure**: Complex features revealed contextually, reducing cognitive load

### 1.4 Reference Implementation

**WRAITH-Transfer** serves as the canonical reference implementation for:
- Base color palette and theming
- Core component patterns
- State management architecture
- Accessibility patterns

All new WRAITH applications should derive their design language from WRAITH-Transfer while adapting for their specific use cases.

---

## 1.5 Standardization Status

This section documents the current UI/UX standardization status across all WRAITH client applications.

### Standardized Clients (as of v1.1.0)

| Client | Standardization | Notes |
|--------|-----------------|-------|
| WRAITH-Transfer | Complete | Reference implementation, Tailwind v4 with `@import "tailwindcss"` |
| WRAITH-Chat | Complete | Uses pink accent (`#ec4899`) for notifications instead of cyan |
| WRAITH-Sync | Complete | Full standardization with ARIA accessibility |
| WRAITH-Android | Partial | Native Kotlin/Compose, follows color palette |
| WRAITH-iOS | Partial | Native Swift/SwiftUI, follows color palette |

### Verified Working Patterns

The following patterns have been verified across all standardized desktop clients:

1. **Color Palette Alignment**: All three desktop clients use the same primary/secondary/background colors
2. **CSS Architecture**: All use modern Tailwind v4 with `@import "tailwindcss"` and `@theme` blocks
3. **Modal Pattern**: Standardized backdrop, container, and accessibility attributes
4. **Text Color Hierarchy**: Consistent use of `text-white`, `text-slate-*` scale
5. **Border Colors**: Standardized `border-slate-700/600/500` hierarchy
6. **Focus States**: Consistent `focus-visible:ring-2 focus-visible:ring-cyan-500` pattern
7. **Icon Button Accessibility**: All icon-only buttons have `aria-label` attributes

### Client-Specific Variations

| Client | Variation | Reason |
|--------|-----------|--------|
| WRAITH-Chat | Accent: `#ec4899` (pink) | Differentiates notifications/badges from transfer progress |
| WRAITH-Sync | Checkbox accent color | Uses CSS custom property for form controls |

---

## 2. Design Strategy

### 2.1 Core Principles

#### 2.1.1 Security-Conscious Design

Visual cues that reinforce security:
- Lock icons for encrypted connections
- Status indicators for connection health
- Clear visual distinction between secure and non-secure states
- Truncated identifiers with copy-to-clipboard functionality

```tsx
// Example: Secure identifier display pattern from WRAITH-Transfer
<div
  onClick={handleCopyNodeId}
  className="text-sm text-slate-400 cursor-pointer hover:text-slate-200 transition-colors"
  title={`${status.node_id}\nClick to copy`}
>
  <span className="text-slate-500">Node: </span>
  <span className="font-mono">
    {status.node_id.slice(0, 12)}...{status.node_id.slice(-4)}
  </span>
</div>
```

#### 2.1.2 Privacy-Focused Patterns

- Minimal data exposure in UI elements
- No analytics or tracking visual indicators
- Clear privacy state communication
- View-once and ephemeral content indicators (inspired by Signal)

#### 2.1.3 Performance-Oriented

- Lightweight UI components
- Efficient re-renders through proper state management
- Progressive loading with skeleton states
- Optimized animations that respect `prefers-reduced-motion`

### 2.2 User Personas

| Persona | Description | Key Needs |
|---------|-------------|-----------|
| Privacy Advocate | Values security over convenience | Clear security indicators, minimal data exposure |
| Power User | Technical, wants advanced features | Keyboard shortcuts, detailed status information |
| Casual User | Prioritizes ease of use | Simple workflows, clear feedback, helpful empty states |
| Enterprise User | Requires compliance features | Audit trails, admin controls, batch operations |

### 2.3 Design Goals and Metrics

| Goal | Metric | Target |
|------|--------|--------|
| Learnability | Time to first successful transfer | < 2 minutes |
| Efficiency | Steps to complete common tasks | Minimize by 20% |
| Error Prevention | User error rate | < 5% for critical operations |
| Accessibility | WCAG compliance | 100% AA criteria |
| Performance | Time to interactive | < 1.5 seconds |

---

## 3. Visual Design System

### 3.1 Color Palette

#### 3.1.1 WRAITH Brand Colors

WRAITH applications use a carefully curated dark-first color palette that conveys security and professionalism.

**WRAITH-Transfer (Reference Palette):**
```css
@theme {
  /* Primary brand colors */
  --color-wraith-primary: #7c3aed;    /* Violet 600 - Primary actions */
  --color-wraith-secondary: #4f46e5;  /* Indigo 600 - Secondary actions */
  --color-wraith-accent: #06b6d4;     /* Cyan 500 - Highlights, progress */
  --color-wraith-dark: #0f172a;       /* Slate 900 - Primary background */
  --color-wraith-darker: #020617;     /* Slate 950 - Deepest background */
}
```

**WRAITH-Chat (Variation):**
```css
@theme {
  /* Chat-specific brand colors */
  --color-wraith-primary: #6366f1;    /* Indigo 500 - Primary actions */
  --color-wraith-secondary: #8b5cf6;  /* Violet 500 - Secondary actions */
  --color-wraith-accent: #ec4899;     /* Pink 500 - Notifications, badges */
  --color-wraith-dark: #1e1b4b;       /* Indigo 950 - Primary background */
  --color-wraith-darker: #0f0d2e;     /* Custom deep indigo */
}
```

#### 3.1.2 Semantic Colors

```css
@theme {
  /* Status indicators */
  --color-success: #22c55e;   /* Green 500 - Connected, completed */
  --color-warning: #f59e0b;   /* Amber 500 - Caution, pending */
  --color-error: #ef4444;     /* Red 500 - Errors, disconnected */

  /* Background hierarchy */
  --color-bg-primary: #0f172a;    /* Main content area */
  --color-bg-secondary: #1e293b;  /* Sidebars, headers, cards */
  --color-bg-tertiary: #334155;   /* Elevated elements, hovers */
}
```

#### 3.1.3 Color Usage Guidelines

| Color | Usage | Example |
|-------|-------|---------|
| `wraith-primary` | Primary buttons, active states | Start Node button |
| `wraith-secondary` | Secondary actions, hover states | Button hover |
| `wraith-accent` | Progress bars, highlights | Transfer progress |
| `success` | Connected status, completed transfers | Status dot |
| `warning` | Initializing, pending states | Status indicator |
| `error` | Errors, disconnected, cancel actions | Stop Node button |

#### 3.1.4 Text Color Hierarchy (Standardized)

The following text color hierarchy has been standardized across all WRAITH clients:

| Class | Tailwind | Usage |
|-------|----------|-------|
| `text-white` | `#ffffff` | Headers, primary text, important labels |
| `text-slate-200` | `#e2e8f0` | Body text, default content |
| `text-slate-300` | `#cbd5e1` | Labels, secondary headers |
| `text-slate-400` | `#94a3b8` | Secondary/muted text, helper text |
| `text-slate-500` | `#64748b` | Captions, hints, timestamps |

**Implementation Example:**
```tsx
<h2 className="text-xl font-semibold text-white">Section Title</h2>
<p className="text-slate-200">Primary body content goes here.</p>
<label className="text-sm font-medium text-slate-300">Input Label</label>
<p className="text-slate-400">Secondary information or help text.</p>
<span className="text-xs text-slate-500">Last updated: 2 hours ago</span>
```

#### 3.1.5 Border Color Hierarchy (Standardized)

Consistent border colors across all clients:

| Class | Tailwind | Usage |
|-------|----------|-------|
| `border-slate-700` | `#334155` | Card/section borders, dividers |
| `border-slate-600` | `#475569` | Input field borders (default state) |
| `border-slate-500` | `#64748b` | Hover state borders |

**Implementation Example:**
```tsx
// Card with standard border
<div className="bg-bg-secondary rounded-lg p-4 border border-slate-700">
  {/* Card content */}
</div>

// Input with border hierarchy
<input
  className="bg-bg-primary border border-slate-600 hover:border-slate-500 rounded-lg"
/>
```

### 3.2 Typography Scale

#### 3.2.1 Font Stack

```css
font-family: system-ui, -apple-system, BlinkMacSystemFont,
             'Segoe UI', Roboto, 'Oxygen', 'Ubuntu',
             'Cantarell', 'Fira Sans', 'Droid Sans',
             'Helvetica Neue', sans-serif;
```

#### 3.2.2 Type Scale

| Level | Size | Weight | Line Height | Usage |
|-------|------|--------|-------------|-------|
| H1 | 2xl (1.5rem) | Bold (700) | 2rem | Page titles |
| H2 | xl (1.25rem) | Semibold (600) | 1.75rem | Section headers |
| H3 | lg (1.125rem) | Medium (500) | 1.5rem | Card titles |
| Body | base (1rem) | Normal (400) | 1.5rem | Primary content |
| Small | sm (0.875rem) | Normal (400) | 1.25rem | Secondary text |
| Micro | xs (0.75rem) | Normal (400) | 1rem | Timestamps, hints |

#### 3.2.3 Monospace Text

Use `font-mono` for:
- Node IDs and peer IDs
- File paths
- Technical data (hashes, keys)
- Code snippets

```tsx
<span className="font-mono text-sm">
  {nodeId.slice(0, 16)}...
</span>
```

### 3.3 Spacing System

Based on Tailwind's default spacing scale (4px base unit):

| Token | Value | Usage |
|-------|-------|-------|
| 1 | 0.25rem (4px) | Minimal spacing |
| 2 | 0.5rem (8px) | Compact elements |
| 3 | 0.75rem (12px) | Standard padding |
| 4 | 1rem (16px) | Section padding |
| 6 | 1.5rem (24px) | Major sections |
| 8 | 2rem (32px) | Large gaps |

**Consistent Spacing Patterns:**
```tsx
// Card padding
className="p-4"  // 16px padding

// Section spacing
className="space-y-4"  // 16px vertical gap

// Icon gaps
className="gap-2"  // 8px gap between icon and text
```

### 3.4 Iconography Guidelines

#### 3.4.1 Icon Style

- Use inline SVG icons for performance and flexibility
- Stroke-based icons with `strokeWidth={2}`
- Consistent sizing: `w-4 h-4` (small), `w-5 h-5` (medium), `w-6 h-6` (large)
- Use `currentColor` for fill/stroke to inherit text color

#### 3.4.2 Icon Component Pattern

```tsx
function SettingsIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0..."
      />
    </svg>
  );
}
```

#### 3.4.3 Icon Library Recommendations

For new icons, prefer:
1. Heroicons (used in current implementations)
2. Lucide React (MIT licensed, consistent with Heroicons style)
3. Custom SVG icons following the same design language

### 3.5 Shadow and Elevation

Minimal shadow usage to maintain the flat, secure aesthetic:

| Level | Class | Usage |
|-------|-------|-------|
| None | - | Most elements |
| Subtle | `shadow-sm` | Dropdowns |
| Medium | `shadow-md` | Modals |
| Large | `shadow-lg` | Floating elements |

### 3.6 Border Radius Standards

| Element | Radius | Class |
|---------|--------|-------|
| Buttons | 8px | `rounded-lg` |
| Cards | 8px | `rounded-lg` |
| Modals | 12px | `rounded-xl` |
| Inputs | 8px | `rounded-lg` |
| Badges | Full | `rounded-full` |
| Avatar | Full | `rounded-full` |
| Progress bars | Full | `rounded-full` |
| Message bubbles | 16px | `rounded-2xl` |

---

## 4. Component Library

### 4.1 Buttons

#### 4.1.1 Button Variants (Verified Working)

These patterns have been tested and verified across WRAITH-Transfer, WRAITH-Chat, and WRAITH-Sync:

**Primary Button:**
```tsx
<button className="px-4 py-2 bg-violet-600 hover:bg-violet-700 text-white rounded-lg font-medium transition-colors">
  Primary Action
</button>
```

**Secondary Button:**
```tsx
<button className="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-lg font-medium transition-colors">
  Secondary Action
</button>
```

**Danger Button:**
```tsx
<button className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg font-medium transition-colors">
  Destructive Action
</button>
```

**Ghost Button:**
```tsx
<button className="px-4 py-2 text-slate-400 hover:text-white transition-colors">
  Cancel
</button>
```

**Icon Button (with required accessibility):**
```tsx
<button
  onClick={handleAction}
  className="p-2 text-slate-400 hover:text-white transition-colors"
  aria-label="Open settings"  /* REQUIRED for icon-only buttons */
>
  <SettingsIcon className="w-5 h-5" />
</button>
```

**Note:** The `title` attribute is optional but `aria-label` is **mandatory** for all icon-only buttons to meet WCAG accessibility requirements.

#### 4.1.2 Button States (Standardized)

| State | Classes |
|-------|---------|
| Default | `bg-violet-600 text-white` |
| Hover | `hover:bg-violet-700` |
| Focus | `focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-500` |
| Disabled | `opacity-50 cursor-not-allowed` |
| Loading | `opacity-50 cursor-not-allowed` + spinner |

**Focus State Pattern:**
```tsx
className="focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-500"
```

This focus pattern uses cyan (`#06b6d4`) as the focus indicator across all clients for consistency, regardless of the client's accent color.

#### 4.1.3 Button Implementation

```tsx
interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  disabled?: boolean;
  children: React.ReactNode;
  onClick?: () => void;
}

function Button({
  variant = 'primary',
  size = 'md',
  loading,
  disabled,
  children,
  onClick
}: ButtonProps) {
  const baseClasses = "rounded-lg font-medium transition-colors";

  const variantClasses = {
    primary: "bg-wraith-primary hover:bg-wraith-secondary text-white",
    secondary: "bg-bg-tertiary hover:bg-slate-600 text-white",
    danger: "bg-red-600 hover:bg-red-700 text-white",
    ghost: "text-slate-400 hover:text-white",
  };

  const sizeClasses = {
    sm: "px-3 py-1.5 text-sm",
    md: "px-4 py-2 text-sm",
    lg: "px-6 py-2.5 text-base",
  };

  const isDisabled = disabled || loading;

  return (
    <button
      onClick={onClick}
      disabled={isDisabled}
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${
        isDisabled ? 'opacity-50 cursor-not-allowed' : ''
      }`}
    >
      {loading ? 'Loading...' : children}
    </button>
  );
}
```

#### 4.1.4 Icon Button Accessibility Requirements

All icon-only buttons **must** have an `aria-label` attribute for screen reader accessibility. This is a mandatory requirement verified across all WRAITH clients:

```tsx
// Correct - with aria-label
<button
  onClick={handlePause}
  className="p-2 text-slate-400 hover:text-white transition-colors"
  aria-label="Pause synchronization"
>
  <PauseIcon className="w-5 h-5" />
</button>

// Correct - with aria-label and title for tooltip
<button
  onClick={handleSettings}
  className="p-2 text-slate-400 hover:text-white transition-colors"
  aria-label="Open settings"
  title="Settings"
>
  <SettingsIcon className="w-5 h-5" />
</button>

// INCORRECT - missing aria-label (accessibility violation)
<button onClick={handleClose} className="p-2 text-slate-400 hover:text-white">
  <CloseIcon className="w-5 h-5" />  {/* Screen readers cannot describe this button */}
</button>
```

**Real-world example from WRAITH-Sync FolderList.tsx:**
```tsx
<button
  onClick={(e) => {
    e.stopPropagation();
    forceSyncFolder(folder.id);
  }}
  className="p-1.5 rounded hover:bg-bg-tertiary transition-colors"
  title="Force Sync"
  aria-label="Force sync folder"
>
  <RefreshIcon className="w-4 h-4" />
</button>
```

### 4.2 Input Fields

#### 4.2.1 Text Input (Standardized Pattern)

This is the verified working input pattern used across all WRAITH clients:

```tsx
<input
  type="text"
  value={value}
  onChange={(e) => setValue(e.target.value)}
  className="w-full px-3 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-cyan-500"
  placeholder="Enter value..."
/>
```

**Key standardization points:**
- Background: `bg-slate-700` (not `bg-bg-primary` for consistency)
- Border: `border-slate-600` (standard input border)
- Placeholder: `placeholder-slate-400` (visible but muted)
- Focus: `focus:ring-2 focus:ring-cyan-500` (consistent cyan ring)

#### 4.2.2 Input with Label and Helper Text

```tsx
<div>
  <label className="block text-sm font-medium text-slate-300 mb-1">
    Peer ID
  </label>
  <input
    type="text"
    placeholder="Enter 64-character hex peer ID"
    className="w-full bg-bg-primary border border-slate-600 rounded-lg px-3 py-2
               text-white placeholder-slate-500 font-mono text-sm
               focus:outline-none focus:border-wraith-primary"
    aria-invalid={!!error}
    aria-describedby={error ? 'peer-id-error' : undefined}
  />
  {value.length > 0 && (
    <div className="mt-1 text-xs text-slate-500">
      {value.length}/64 characters
    </div>
  )}
</div>
```

#### 4.2.3 Input with Error State

```tsx
<input
  type="text"
  className={`w-full bg-bg-primary border rounded-lg px-3 py-2
              text-white placeholder-slate-500 focus:outline-none ${
    error
      ? 'border-red-500 focus:border-red-500'
      : 'border-slate-600 focus:border-wraith-primary'
  }`}
  aria-invalid={!!error}
/>
{error && (
  <div className="mt-1 text-xs text-red-400">{error}</div>
)}
```

#### 4.2.4 Search Input

```tsx
<div className="relative">
  <SearchIcon className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
  <input
    type="text"
    placeholder="Search conversations..."
    className="w-full bg-bg-primary border border-slate-600 rounded-lg
               pl-10 pr-4 py-2 text-sm text-white placeholder-slate-500
               focus:outline-none focus:border-wraith-primary"
  />
</div>
```

#### 4.2.5 Textarea (Auto-resize)

```tsx
function AutoResizeTextarea({ value, onChange, placeholder }: Props) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      textareaRef.current.style.height =
        `${Math.min(textareaRef.current.scrollHeight, 150)}px`;
    }
  }, [value]);

  return (
    <textarea
      ref={textareaRef}
      value={value}
      onChange={onChange}
      placeholder={placeholder}
      rows={1}
      className="w-full bg-bg-primary border border-slate-600 rounded-lg
                 px-4 py-2.5 text-white placeholder-slate-500
                 focus:outline-none focus:border-wraith-primary resize-none"
      style={{ minHeight: '44px', maxHeight: '150px' }}
    />
  );
}
```

### 4.3 Cards and Containers

#### 4.3.1 Basic Card

```tsx
<div className="bg-bg-secondary rounded-lg p-4 border border-slate-700">
  {/* Card content */}
</div>
```

#### 4.3.2 Interactive Card

```tsx
<button
  onClick={onClick}
  className={`w-full p-3 flex items-center gap-3 transition-colors text-left ${
    isSelected
      ? 'bg-wraith-primary/20 border-l-2 border-wraith-primary'
      : 'hover:bg-bg-primary border-l-2 border-transparent'
  }`}
>
  {/* Card content */}
</button>
```

#### 4.3.3 Transfer Item Card (Reference Implementation)

```tsx
<div className="bg-bg-secondary rounded-lg p-4 border border-slate-700">
  <div className="flex items-center justify-between mb-2">
    <div className="flex items-center gap-3">
      <div className={`text-lg ${direction === 'upload' ? 'rotate-180' : ''}`}>
        {direction === 'upload' ? '↑' : '↓'}
      </div>
      <div>
        <div className="font-medium text-white">{fileName}</div>
        <div className="text-sm text-slate-400 font-mono">
          {peerId.slice(0, 16)}...
        </div>
      </div>
    </div>
    <div className="flex items-center gap-4">
      <span className={`text-sm ${statusColors[status]}`}>
        {status.replace('_', ' ')}
      </span>
      {isActive && (
        <button
          onClick={onCancel}
          className="text-slate-400 hover:text-red-500 transition-colors"
        >
          ✕
        </button>
      )}
    </div>
  </div>
  {/* Progress section */}
</div>
```

### 4.4 Modal/Dialog Patterns

#### 4.4.1 Modal Container (Standardized)

This is the verified working modal pattern used across WRAITH-Transfer, WRAITH-Chat, and WRAITH-Sync:

```tsx
<div
  className="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
  onClick={onClose}
>
  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="modal-title"
    className="bg-slate-800 rounded-xl p-6 w-full max-w-md"
    onClick={(e) => e.stopPropagation()}
  >
    <h2 id="modal-title" className="text-xl font-semibold text-white mb-4">
      Modal Title
    </h2>
    {/* Modal content */}
    <div className="flex justify-end gap-3 mt-6">
      <button
        onClick={onClose}
        className="px-4 py-2 text-slate-400 hover:text-white transition-colors"
      >
        Cancel
      </button>
      <button className="px-4 py-2 bg-violet-600 hover:bg-violet-700 rounded-lg text-white font-medium transition-colors">
        Confirm
      </button>
    </div>
  </div>
</div>
```

**Key standardization points:**
- Backdrop: `bg-black/60` (60% opacity for better content visibility)
- Container: `bg-slate-800` (consistent dark background)
- ARIA attributes on the inner container (`role="dialog"`, `aria-modal="true"`, `aria-labelledby`)
- Click propagation stopped on inner container
- Close on backdrop click (outer div)
- Title linked via `aria-labelledby` and matching `id`

#### 4.4.2 Full Settings Modal (Scrollable)

```tsx
<div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
  <div className="bg-bg-secondary rounded-xl border border-slate-700 w-full max-w-2xl max-h-[90vh] overflow-auto">
    {/* Sticky header */}
    <div className="sticky top-0 bg-bg-secondary border-b border-slate-700 px-6 py-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-white">Settings</h2>
        <button onClick={onClose} className="text-slate-400 hover:text-white">
          ✕
        </button>
      </div>
    </div>

    {/* Scrollable content */}
    <div className="p-6 space-y-6">
      {/* Settings sections */}
    </div>

    {/* Sticky footer */}
    <div className="sticky bottom-0 bg-bg-secondary border-t border-slate-700 px-6 py-4">
      <div className="flex justify-between">
        <button className="text-slate-400 hover:text-white">Reset to Defaults</button>
        <div className="flex gap-3">
          <button className="text-slate-400 hover:text-white">Cancel</button>
          <button className="px-6 py-2 bg-wraith-primary rounded-lg text-white font-medium">
            Save
          </button>
        </div>
      </div>
    </div>
  </div>
</div>
```

### 4.5 Navigation Patterns

#### 4.5.1 Header

```tsx
<header className="bg-bg-secondary border-b border-slate-700 px-6 py-4">
  <div className="flex items-center justify-between">
    {/* Left: Logo and status */}
    <div className="flex items-center gap-4">
      <h1 className="text-xl font-bold text-white">WRAITH Transfer</h1>
      <div className="flex items-center gap-2">
        <div className={`w-2 h-2 rounded-full ${isRunning ? 'bg-green-500' : 'bg-red-500'}`} />
        <span className="text-sm text-slate-400">
          {isRunning ? 'Connected' : 'Disconnected'}
        </span>
      </div>
    </div>

    {/* Right: Actions */}
    <div className="flex items-center gap-4">
      {/* Status info, buttons */}
    </div>
  </div>
</header>
```

#### 4.5.2 Sidebar

```tsx
<div className="w-80 bg-bg-secondary border-r border-slate-700 flex flex-col h-full">
  {/* Search and actions */}
  <div className="p-4 space-y-3 border-b border-slate-700">
    {/* Search input */}
    {/* Action buttons */}
  </div>

  {/* Filter tabs */}
  <div className="flex border-b border-slate-700">
    <FilterTab label="All" count={total} active={filter === 'all'} />
    <FilterTab label="Direct" count={direct} active={filter === 'direct'} />
    <FilterTab label="Groups" count={groups} active={filter === 'groups'} />
  </div>

  {/* Scrollable list */}
  <div className="flex-1 overflow-y-auto">
    {/* List items */}
  </div>
</div>
```

#### 4.5.3 Filter Tabs

```tsx
function FilterTab({ label, count, active, onClick }: FilterTabProps) {
  return (
    <button
      onClick={onClick}
      className={`flex-1 py-2.5 text-sm font-medium transition-colors relative ${
        active ? 'text-wraith-primary' : 'text-slate-400 hover:text-slate-200'
      }`}
    >
      {label}
      {count > 0 && (
        <span className={`ml-1.5 text-xs ${active ? 'text-wraith-primary' : 'text-slate-500'}`}>
          ({count})
        </span>
      )}
      {active && (
        <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-wraith-primary" />
      )}
    </button>
  );
}
```

### 4.6 Status Indicators

#### 4.6.1 Connection Status Dot

```tsx
<div className={`w-2 h-2 rounded-full ${
  status === 'connected' ? 'bg-green-500' :
  status === 'connecting' ? 'bg-yellow-500 animate-pulse' :
  status === 'disconnecting' ? 'bg-orange-500 animate-pulse' :
  'bg-red-500'
}`} />
```

#### 4.6.2 Status Text Colors

```tsx
const statusColors: Record<string, string> = {
  initializing: 'text-yellow-500',
  in_progress: 'text-blue-500',
  completed: 'text-green-500',
  failed: 'text-red-500',
  cancelled: 'text-slate-500',
};
```

#### 4.6.3 Message Status Indicators (Chat)

```tsx
function MessageStatus({ message }: { message: Message }) {
  if (message.read_by_me) {
    return (
      <span className="flex text-blue-300" title="Read">
        <CheckIcon className="w-3.5 h-3.5" />
        <CheckIcon className="w-3.5 h-3.5 -ml-2" />
      </span>
    );
  }
  if (message.delivered) {
    return (
      <span className="flex" title="Delivered">
        <CheckIcon className="w-3.5 h-3.5" />
        <CheckIcon className="w-3.5 h-3.5 -ml-2" />
      </span>
    );
  }
  if (message.sent) {
    return <CheckIcon className="w-3.5 h-3.5" title="Sent" />;
  }
  return <ClockIcon className="w-3.5 h-3.5" title="Sending..." />;
}
```

### 4.7 Progress Indicators

#### 4.7.1 Determinate Progress Bar

```tsx
<div className="space-y-1">
  <div className="flex justify-between text-sm text-slate-400">
    <span>{formatBytes(transferred)} / {formatBytes(total)}</span>
    <span>{progressPercent}%</span>
  </div>
  <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
    <div
      className={`h-full transition-all duration-300 ${
        status === 'completed' ? 'bg-green-500' :
        status === 'failed' ? 'bg-red-500' :
        'bg-wraith-accent'
      }`}
      style={{ width: `${progressPercent}%` }}
    />
  </div>
  {isActive && (
    <div className="flex justify-between text-xs text-slate-500">
      <span>{speed > 0 ? formatSpeed(speed) : 'Calculating...'}</span>
      <span>ETA: {eta > 0 ? formatETA(eta) : '--:--'}</span>
    </div>
  )}
</div>
```

#### 4.7.2 Transfer Speed and ETA Formatting

```tsx
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function formatSpeed(bytesPerSecond: number): string {
  return formatBytes(bytesPerSecond) + '/s';
}

function formatETA(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return '--:--';
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);
  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  }
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}
```

### 4.8 Toast/Notification Patterns

#### 4.8.1 Inline Toast

```tsx
<span className="absolute -top-8 left-1/2 -translate-x-1/2 bg-green-600 text-white text-xs px-2 py-1 rounded whitespace-nowrap">
  Copied to clipboard!
</span>
```

#### 4.8.2 Error Banner

```tsx
<div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-400 text-sm">
  {errorMessage}
</div>
```

#### 4.8.3 System Message (Chat)

```tsx
<div className="flex justify-center my-4">
  <span className="px-3 py-1 bg-bg-tertiary/50 rounded-full text-xs text-slate-400">
    {text}
  </span>
</div>
```

### 4.9 Toggle/Switch Component

```tsx
<button
  onClick={() => setEnabled(!enabled)}
  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
    enabled ? 'bg-wraith-primary' : 'bg-slate-600'
  }`}
  role="switch"
  aria-checked={enabled}
>
  <span
    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
      enabled ? 'translate-x-6' : 'translate-x-1'
    }`}
  />
</button>
```

### 4.10 Avatar Component

#### 4.10.1 Text Avatar

```tsx
<div className="w-12 h-12 rounded-full bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center text-lg font-semibold text-white">
  {initial}
</div>
```

#### 4.10.2 Group Avatar

```tsx
<div className="w-12 h-12 rounded-full bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center">
  <UsersIcon className="w-5 h-5 text-white" />
</div>
```

#### 4.10.3 Avatar with Online Status

```tsx
<div className="relative">
  <div className="w-12 h-12 rounded-full bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center text-lg font-semibold text-white">
    {initial}
  </div>
  <div className="absolute bottom-0 right-0 w-3 h-3 bg-green-500 rounded-full border-2 border-bg-secondary" />
</div>
```

---

## 5. Layout Principles

### 5.1 Grid System

WRAITH applications use a flexible layout system based on:
- Flexbox for component-level layouts
- CSS Grid for complex dashboard layouts (where needed)

#### 5.1.1 Main Application Layout

```tsx
<div className="h-screen bg-bg-primary text-slate-200 flex flex-col">
  <Header />
  <main className="flex-1 flex overflow-hidden">
    <Sidebar />
    <ContentArea />
    <InfoPanel /> {/* Conditional */}
  </main>
  <StatusBar />
</div>
```

### 5.2 Responsive Breakpoints

While Tauri applications have more control over window size, support these breakpoints for window resizing:

| Breakpoint | Width | Layout Adaptation |
|------------|-------|-------------------|
| Compact | < 640px | Single column, collapsed sidebar |
| Default | 640-1024px | Two column with sidebar |
| Wide | > 1024px | Three column with info panel |

### 5.3 Sidebar Patterns

**WRAITH-Transfer Pattern (Right Sidebar):**
```tsx
<main className="flex-1 flex overflow-hidden">
  <TransferList />  {/* Flex-grow content */}
  <SessionPanel />  {/* Fixed-width right sidebar: w-72 */}
</main>
```

**WRAITH-Chat Pattern (Left Sidebar + Optional Right):**
```tsx
<main className="flex-1 flex overflow-hidden">
  <Sidebar />       {/* Fixed-width left sidebar: w-80 */}
  <ChatArea />      {/* Flex-grow content */}
  {showInfoPanel && <InfoPanel />}  {/* Conditional right panel */}
</main>
```

### 5.4 Content Area Organization

#### 5.4.1 List View Pattern

```tsx
<div className="flex-1 overflow-auto p-4 space-y-3">
  {items.map((item) => (
    <ListItem key={item.id} item={item} />
  ))}
</div>
```

#### 5.4.2 Empty State Pattern

```tsx
<div className="flex-1 flex items-center justify-center text-slate-500">
  <div className="text-center">
    <div className="text-4xl mb-2">Icon</div>
    <div>Primary message</div>
    <div className="text-sm">Secondary message with guidance</div>
  </div>
</div>
```

### 5.5 Header/Footer Patterns

**Header Structure:**
```tsx
<header className="bg-bg-secondary border-b border-slate-700 px-6 py-4">
  {/* Fixed height, flex layout */}
</header>
```

**Footer/Status Bar Structure:**
```tsx
<footer className="bg-bg-secondary border-t border-slate-700 px-6 py-3">
  {/* Fixed height, flex layout */}
</footer>
```

---

## 6. Interaction Patterns

### 6.1 Hover States

All interactive elements should have clear hover states:

```tsx
// Button hover
className="hover:bg-wraith-secondary"

// Text hover
className="hover:text-white"

// Background hover
className="hover:bg-bg-primary"
```

### 6.2 Focus States

Consistent focus indicators for accessibility:

```css
:focus-visible {
  outline: 2px solid var(--color-wraith-accent);
  outline-offset: 2px;
}
```

For custom focus styling:
```tsx
className="focus:outline-none focus:border-wraith-primary"
```

### 6.3 Loading States

#### 6.3.1 Button Loading

```tsx
<button disabled={loading} className={loading ? 'opacity-50 cursor-not-allowed' : ''}>
  {loading ? 'Loading...' : 'Action'}
</button>
```

#### 6.3.2 Pulse Animation for Pending States

```tsx
<div className="bg-yellow-500 animate-pulse" />
```

### 6.4 Error States

#### 6.4.1 Form Validation Errors

```tsx
<input
  className={error ? 'border-red-500' : 'border-slate-600'}
  aria-invalid={!!error}
/>
{error && <div className="text-xs text-red-400 mt-1">{error}</div>}
```

#### 6.4.2 Operation Errors

```tsx
{error && (
  <div className="bg-red-500/20 text-red-400 px-4 py-2 rounded mb-4">
    {error}
  </div>
)}
```

### 6.5 Empty States

Provide helpful empty states with:
- Visual indicator (icon)
- Primary message explaining the state
- Secondary message with guidance
- Optional call-to-action

```tsx
<div className="h-full flex items-center justify-center">
  <div className="text-center max-w-md px-4">
    <div className="w-24 h-24 rounded-full bg-gradient-to-br from-wraith-primary to-wraith-secondary flex items-center justify-center mx-auto mb-6">
      <ChatIcon className="w-12 h-12 text-white" />
    </div>
    <h2 className="text-2xl font-semibold text-white mb-2">
      Welcome to WRAITH Chat
    </h2>
    <p className="text-slate-400 mb-6">
      Select a conversation from the sidebar or start a new chat.
    </p>
    <button className="px-6 py-2.5 bg-wraith-primary hover:bg-wraith-secondary rounded-lg text-white font-medium">
      Start New Chat
    </button>
  </div>
</div>
```

### 6.6 Transitions and Animations

#### 6.6.1 Standard Transition

```tsx
className="transition-colors"  // Color changes
className="transition-all"     // All properties
className="transition-transform" // Position/size
```

#### 6.6.2 Duration Guidelines

| Type | Duration | Use Case |
|------|----------|----------|
| Fast | 150ms | Hovers, color changes |
| Normal | 300ms | Most transitions |
| Slow | 500ms | Modal enter/exit |

#### 6.6.3 Progress Bar Animation

```tsx
className="transition-all duration-300"
```

### 6.7 Keyboard Navigation

#### 6.7.1 Modal Keyboard Handling

```tsx
const handleKeyDown = (e: React.KeyboardEvent) => {
  if (e.key === 'Escape') {
    onClose();
  } else if (e.key === 'Enter' && canSubmit) {
    onSubmit();
  }
};
```

#### 6.7.2 Text Input Enter Handler

```tsx
const handleKeyDown = (e: React.KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
};
```

### 6.8 Scroll Behavior

#### 6.8.1 Custom Scrollbar Styling

```css
::-webkit-scrollbar {
  width: 8px;
}

::-webkit-scrollbar-track {
  background: var(--color-bg-secondary);
}

::-webkit-scrollbar-thumb {
  background: var(--color-wraith-primary);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--color-wraith-secondary);
}
```

#### 6.8.2 Auto-scroll to Bottom (Chat)

```tsx
const messagesEndRef = useRef<HTMLDivElement>(null);

useEffect(() => {
  messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
}, [messageCount]);

// In render:
<div ref={messagesEndRef} />
```

---

## 7. Application-Specific Guidelines

### 7.1 WRAITH-Transfer UI Patterns

WRAITH-Transfer serves as the reference implementation. Key patterns:

#### 7.1.1 File Transfer Item

- Direction indicator (upload/download arrow)
- File name and peer ID
- Status badge with semantic colors
- Progress bar with speed and ETA
- Cancel button for active transfers

#### 7.1.2 Session Management Panel

- Fixed-width right sidebar (288px / `w-72`)
- Session count in header
- Session cards with:
  - Status dot with animation
  - Peer ID (truncated with monospace font)
  - Duration timer
  - Bytes sent/received
  - Close button

#### 7.1.3 Node Status Header

- Application title
- Connection status (dot + text)
- Node ID (truncated, click to copy)
- Session/transfer counts
- Theme toggle
- Settings button
- Start/Stop Node button

### 7.2 WRAITH-Chat UI Patterns

WRAITH-Chat extends the base patterns for messaging:

#### 7.2.1 Message Bubbles

- Outgoing messages: `bg-wraith-primary` aligned right
- Incoming messages: `bg-bg-tertiary` aligned left
- Rounded corners: `rounded-2xl` with tail (`rounded-br-md` or `rounded-bl-md`)
- Timestamp and status indicators in footer
- Context menu on hover (reply, react, more)

#### 7.2.2 Conversation List

- Avatar with initial or group icon
- Online status indicator
- Name and last message preview
- Timestamp and unread badge
- Selected state with left border accent

#### 7.2.3 Chat Header

- Back button (mobile)
- Avatar and name
- Status indicator
- Action buttons (voice call, video call, info)

#### 7.2.4 Message Input

- Attachment button
- Auto-resizing textarea
- Emoji picker toggle
- Send button (when text present) / Voice button (when empty)
- Character count and keyboard hint

#### 7.2.5 Voice/Video Call Overlays

- Full-screen overlay (`fixed inset-0`)
- Caller avatar and identifier
- Call state text (ringing, connecting, duration)
- Call quality indicators
- Control buttons (mute, end call, speaker, video toggle)
- Audio device settings panel

### 7.3 WRAITH-Sync UI Patterns

WRAITH-Sync combines file management with synchronization status:

#### 7.3.1 Sync Status Indicators

- Overall sync status (synced, syncing, pending)
- Per-file sync status
- Conflict indicators
- Version history access

#### 7.3.2 File Browser

- Directory tree navigation
- File list with sync status
- Right-click context menu
- Drag-and-drop support

### 7.4 Common Patterns Across All Apps

#### 7.4.1 Settings Modal Structure

Consistent settings organization:
1. Appearance (theme selection)
2. General (app-specific settings)
3. Network (port, connection settings)
4. Security (encryption, key management)
5. About (version, links)

#### 7.4.2 Peer ID Display

Always use this pattern for peer IDs:
```tsx
<span className="font-mono text-sm text-slate-400">
  {peerId.slice(0, 16)}...
</span>
```

#### 7.4.3 Copy to Clipboard

```tsx
const handleCopy = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  } catch (err) {
    console.error('Failed to copy:', err);
  }
};
```

---

## 8. Technical Implementation

### 8.1 Tailwind CSS Configuration

#### 8.1.1 Base Configuration Pattern

```css
/* index.css */
@import "tailwindcss";

@theme {
  /* WRAITH brand colors */
  --color-wraith-primary: #7c3aed;
  --color-wraith-secondary: #4f46e5;
  --color-wraith-accent: #06b6d4;
  --color-wraith-dark: #0f172a;
  --color-wraith-darker: #020617;

  /* Status colors */
  --color-success: #22c55e;
  --color-warning: #f59e0b;
  --color-error: #ef4444;

  /* Background hierarchy */
  --color-bg-primary: #0f172a;
  --color-bg-secondary: #1e293b;
  --color-bg-tertiary: #334155;
}

/* Base styles */
body {
  margin: 0;
  min-width: 320px;
  min-height: 100vh;
  background-color: var(--color-bg-primary);
  color: #e2e8f0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont,
               'Segoe UI', Roboto, sans-serif;
}

/* Focus styles for accessibility */
:focus-visible {
  outline: 2px solid var(--color-wraith-accent);
  outline-offset: 2px;
}
```

### 8.1.2 Tailwind v4 CSS Architecture

All standardized WRAITH clients use Tailwind v4 with the modern `@import` directive and `@theme` blocks:

```css
/* Modern Tailwind v4 pattern (WRAITH standard) */
@import "tailwindcss";

@theme {
  --color-wraith-primary: #7c3aed;
  --color-wraith-secondary: #4f46e5;
  --color-wraith-accent: #06b6d4;
  /* ... additional custom properties */
}
```

**Note:** Older Tailwind v3 patterns using `@tailwind base; @tailwind components; @tailwind utilities;` should be migrated to the v4 pattern when updating existing clients.

### 8.1.3 Complete CSS Template

This is the complete, verified CSS template used by WRAITH-Transfer and WRAITH-Sync:

```css
@import "tailwindcss";

/* Custom theme overrides - WRAITH Design System */
@theme {
  /* WRAITH brand colors (aligned with design reference) */
  --color-wraith-primary: #7c3aed;    /* Violet 600 - Primary actions */
  --color-wraith-secondary: #4f46e5;  /* Indigo 600 - Secondary actions */
  --color-wraith-accent: #06b6d4;     /* Cyan 500 - Highlights, progress */
  --color-wraith-dark: #0f172a;       /* Slate 900 - Primary background */
  --color-wraith-darker: #020617;     /* Slate 950 - Deepest background */

  /* Status colors (standardized) */
  --color-success: #22c55e;   /* Green 500 - Connected, completed */
  --color-warning: #f59e0b;   /* Amber 500 - Caution, pending */
  --color-error: #ef4444;     /* Red 500 - Errors, disconnected */

  /* Background colors for dark theme (aligned with design reference) */
  --color-bg-primary: #0f172a;    /* Main content area */
  --color-bg-secondary: #1e293b;  /* Sidebars, headers, cards */
  --color-bg-tertiary: #334155;   /* Elevated elements, hovers */
}

/* Base styles */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body,
#root {
  height: 100%;
  overflow: hidden;
}

body {
  min-width: 320px;
  min-height: 100vh;
  background-color: var(--color-bg-primary);
  color: #e2e8f0;
  font-family: system-ui, -apple-system, BlinkMacSystemFont,
               'Segoe UI', Roboto, 'Oxygen', 'Ubuntu',
               'Cantarell', 'Fira Sans', 'Droid Sans',
               'Helvetica Neue', sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* Focus styles for accessibility */
:focus-visible {
  outline: 2px solid var(--color-wraith-accent);
  outline-offset: 2px;
}

/* Custom scrollbar */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: var(--color-bg-secondary);
}

::-webkit-scrollbar-thumb {
  background: var(--color-wraith-primary);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--color-wraith-secondary);
}
```

**WRAITH-Chat variation:** Replace `--color-wraith-accent: #06b6d4;` with `--color-wraith-accent: #ec4899;` for pink notification accents.

### 8.2 CSS Variables Approach

Benefits of CSS variables over Tailwind config extension:
- Runtime theming capability
- Easier dark/light mode switching
- Better browser DevTools experience
- Consistent with Tailwind v4 patterns

### 8.3 Component Structure

#### 8.3.1 Recommended File Structure

```
frontend/src/
├── components/
│   ├── index.ts           # Re-exports
│   ├── Header.tsx
│   ├── Sidebar.tsx
│   ├── TransferList.tsx
│   ├── NewTransferDialog.tsx
│   └── SettingsPanel.tsx
├── stores/
│   ├── nodeStore.ts
│   ├── transferStore.ts
│   ├── sessionStore.ts
│   └── settingsStore.ts
├── lib/
│   └── tauri.ts           # Tauri API wrappers
├── types/
│   └── index.ts           # TypeScript interfaces
├── App.tsx
├── main.tsx
└── index.css
```

#### 8.3.2 Component Re-exports

```tsx
// components/index.ts
export { Header } from './Header';
export { TransferList } from './TransferList';
export { SessionPanel } from './SessionPanel';
export { NewTransferDialog } from './NewTransferDialog';
export { StatusBar } from './StatusBar';
export { SettingsPanel } from './SettingsPanel';
```

### 8.4 State Management Patterns

#### 8.4.1 Zustand Store Structure

```tsx
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface StoreState {
  // State
  data: DataType[];
  loading: boolean;
  error: string | null;

  // Actions
  fetchData: () => Promise<void>;
  createItem: (item: DataType) => Promise<void>;
  clearError: () => void;
}

export const useStore = create<StoreState>((set, get) => ({
  data: [],
  loading: false,
  error: null,

  fetchData: async () => {
    set({ loading: true, error: null });
    try {
      const data = await api.getData();
      set({ data, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  createItem: async (item) => {
    try {
      await api.createItem(item);
      await get().fetchData(); // Refresh
    } catch (e) {
      set({ error: String(e) });
    }
  },

  clearError: () => set({ error: null }),
}));
```

#### 8.4.2 Persisted Settings Store

```tsx
export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      theme: 'system',
      downloadDir: '',
      maxConcurrentTransfers: 3,

      setTheme: (theme) => set({ theme }),
      setDownloadDir: (dir) => set({ downloadDir: dir }),
      resetToDefaults: () => set(DEFAULT_SETTINGS),
    }),
    {
      name: 'wraith-settings', // localStorage key
    }
  )
);
```

### 8.5 Tauri API Integration

#### 8.5.1 API Wrapper Pattern

```tsx
// lib/tauri.ts
import { invoke } from '@tauri-apps/api/core';

export async function getNodeStatus(): Promise<NodeStatus> {
  return invoke('get_node_status');
}

export async function startNode(): Promise<NodeStatus> {
  return invoke('start_node');
}

export async function stopNode(): Promise<void> {
  return invoke('stop_node');
}

export async function sendFile(peerId: string, filePath: string): Promise<string> {
  return invoke('send_file', { peerId, filePath });
}
```

### 8.6 Performance Considerations

#### 8.6.1 Polling Optimization

```tsx
// Only poll when node is running
useEffect(() => {
  if (!status?.running) return;

  const interval = setInterval(() => {
    fetchStatus();
    fetchTransfers();
  }, 1000);

  return () => clearInterval(interval);
}, [status?.running, fetchStatus, fetchTransfers]);
```

#### 8.6.2 Memoization for Filtered Lists

```tsx
const filteredConversations = useMemo(() => {
  return conversations.filter((conv) => {
    if (filter === 'direct' && conv.conv_type !== 'direct') return false;
    if (searchQuery.trim()) {
      return conv.display_name?.toLowerCase().includes(searchQuery.toLowerCase());
    }
    return true;
  });
}, [conversations, filter, searchQuery]);
```

#### 8.6.3 Ref-based State for Calculations

```tsx
// For speed calculation without triggering re-renders
const prevBytesRef = useRef(transfer.transferred_bytes);
const [prevTime, setPrevTime] = useState(() => Date.now());

useEffect(() => {
  const now = Date.now();
  const timeDiff = (now - prevTime) / 1000;

  if (timeDiff >= 1.0 && transfer.status === 'in_progress') {
    const bytesDiff = transfer.transferred_bytes - prevBytesRef.current;
    setSpeed(bytesDiff / timeDiff);
    prevBytesRef.current = transfer.transferred_bytes;
    setPrevTime(now);
  }
}, [transfer.transferred_bytes, transfer.status, prevTime]);
```

### 8.7 TypeScript Patterns

#### 8.7.1 Interface Definitions

```tsx
// types/index.ts
export interface NodeStatus {
  running: boolean;
  node_id: string | null;
  active_sessions: number;
  active_transfers: number;
}

export interface TransferInfo {
  id: string;
  peer_id: string;
  file_name: string;
  total_bytes: number;
  transferred_bytes: number;
  progress: number;
  status: 'initializing' | 'in_progress' | 'completed' | 'failed' | 'cancelled';
  direction: 'upload' | 'download';
}

export type Theme = 'light' | 'dark' | 'system';
```

#### 8.7.2 Component Props Pattern

```tsx
interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
}

interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  disabled?: boolean;
  children: React.ReactNode;
  onClick?: () => void;
}
```

---

## 9. Accessibility Standards

### 9.1 WCAG 2.1 AA Compliance

WRAITH applications target WCAG 2.1 Level AA compliance. Key requirements:

#### 9.1.1 Perceivable

| Criterion | Requirement | Implementation |
|-----------|-------------|----------------|
| 1.1.1 Non-text Content | Alt text for images | Use `aria-label` for icons |
| 1.3.1 Info and Relationships | Semantic markup | Use `<header>`, `<main>`, `<nav>` |
| 1.4.3 Contrast (Minimum) | 4.5:1 text contrast | Color palette designed for contrast |
| 1.4.11 Non-text Contrast | 3:1 for UI components | Border and focus states meet ratio |

#### 9.1.2 Operable

| Criterion | Requirement | Implementation |
|-----------|-------------|----------------|
| 2.1.1 Keyboard | All functionality via keyboard | Tab navigation, Enter/Space activation |
| 2.1.2 No Keyboard Trap | Users can navigate freely | Escape closes modals |
| 2.4.3 Focus Order | Logical tab sequence | DOM order matches visual order |
| 2.4.7 Focus Visible | Clear focus indicators | `focus-visible` outline styling |

#### 9.1.3 Understandable

| Criterion | Requirement | Implementation |
|-----------|-------------|----------------|
| 3.2.1 On Focus | No unexpected changes | Focus doesn't trigger actions |
| 3.3.1 Error Identification | Errors clearly described | Error messages with `aria-invalid` |
| 3.3.2 Labels/Instructions | Form fields labeled | `<label>` elements, `aria-describedby` |

#### 9.1.4 Robust

| Criterion | Requirement | Implementation |
|-----------|-------------|----------------|
| 4.1.1 Parsing | Valid HTML | Proper nesting, unique IDs |
| 4.1.2 Name, Role, Value | Accessible names | `aria-label`, semantic elements |

### 9.2 Keyboard Navigation

#### 9.2.1 Focus Management

```tsx
// Modal focus management
useEffect(() => {
  if (isOpen) {
    // Store previously focused element
    previousFocus.current = document.activeElement as HTMLElement;
    // Focus first focusable element in modal
    firstFocusableRef.current?.focus();
  }

  return () => {
    // Return focus when modal closes
    previousFocus.current?.focus();
  };
}, [isOpen]);
```

#### 9.2.2 Keyboard Shortcuts

| Action | Key(s) | Context |
|--------|--------|---------|
| Close modal/dialog | Escape | Any modal |
| Submit form | Enter | Form inputs |
| New line | Shift+Enter | Text areas |
| Navigate list | Arrow Up/Down | List components |
| Toggle theme | - | Planned |

### 9.3 Screen Reader Support

#### 9.3.1 ARIA Attributes

```tsx
// Modal
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby="modal-title"
>
  <h2 id="modal-title">Dialog Title</h2>
</div>

// Toggle switch
<button
  role="switch"
  aria-checked={enabled}
  aria-label="Auto-accept transfers"
/>

// Status indicator
<span aria-live="polite" aria-atomic="true">
  {status} - {connected ? 'Connected' : 'Disconnected'}
</span>
```

#### 9.3.2 Descriptive Labels

```tsx
// Icon buttons must have aria-label
<button
  onClick={onSettings}
  aria-label="Open settings"
  title="Settings"
>
  <SettingsIcon className="w-5 h-5" />
</button>

// Form fields with descriptions
<input
  aria-invalid={!!error}
  aria-describedby={error ? 'field-error' : 'field-hint'}
/>
<div id="field-hint" className="text-xs text-slate-500">
  Enter a 64-character hexadecimal ID
</div>
{error && (
  <div id="field-error" className="text-xs text-red-400" role="alert">
    {error}
  </div>
)}
```

### 9.4 Color Contrast Requirements

| Element | Minimum Ratio | Current Implementation |
|---------|---------------|------------------------|
| Normal text | 4.5:1 | `text-slate-200` on `bg-bg-primary` |
| Large text | 3:1 | Headers meet this ratio |
| UI components | 3:1 | Borders and icons |
| Focus indicators | 3:1 | `wraith-accent` outline |

### 9.5 Focus Management

#### 9.5.1 Visible Focus Style

```css
:focus-visible {
  outline: 2px solid var(--color-wraith-accent);
  outline-offset: 2px;
}
```

#### 9.5.2 Custom Focus for Inputs

```tsx
className="focus:outline-none focus:border-wraith-primary focus:ring-1 focus:ring-wraith-primary"
```

### 9.6 Reduced Motion Support

```tsx
// Respect user preferences
const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

// Conditional animation
className={prefersReducedMotion ? '' : 'animate-pulse'}
```

---

## 10. Future Enhancements

### 10.1 Planned Improvements

#### 10.1.1 Design System Enhancements

| Enhancement | Priority | Description |
|-------------|----------|-------------|
| Component Library Package | High | Extract shared components to npm package |
| Storybook Documentation | High | Visual component documentation |
| Theme Customization | Medium | User-selectable accent colors |
| Animation Library | Medium | Standardized motion design |
| Icon Set Consolidation | Low | Unified icon library |

#### 10.1.2 Accessibility Enhancements

| Enhancement | Priority | Description |
|-------------|----------|-------------|
| Screen Reader Testing | High | Full NVDA/JAWS/VoiceOver audit |
| Focus Trap for Modals | High | Proper focus containment |
| Skip Navigation Links | Medium | Skip to main content |
| High Contrast Mode | Medium | Alternative high contrast theme |
| Reduced Motion Mode | Low | Full animation-free experience |

### 10.2 Theming Extensibility

#### 10.2.1 Dynamic Theme Support

Future architecture for runtime theme switching:

```tsx
// Theme context
interface Theme {
  primary: string;
  secondary: string;
  accent: string;
  background: {
    primary: string;
    secondary: string;
    tertiary: string;
  };
  text: {
    primary: string;
    secondary: string;
    muted: string;
  };
}

// Theme provider
function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { theme } = useSettingsStore();

  useEffect(() => {
    const root = document.documentElement;
    Object.entries(themes[theme]).forEach(([key, value]) => {
      root.style.setProperty(`--color-${key}`, value);
    });
  }, [theme]);

  return children;
}
```

#### 10.2.2 Planned Theme Variants

| Theme | Description | Status |
|-------|-------------|--------|
| Dark (Default) | Current implementation | Production |
| Light | Light mode variant | Planned |
| System | Follows OS preference | Production |
| High Contrast | Accessibility theme | Planned |
| Custom | User-defined colors | Planned |

### 10.3 Platform-Specific Considerations

#### 10.3.1 macOS

- Native window controls integration
- Menu bar integration
- Touch Bar support (where applicable)
- System accent color support

#### 10.3.2 Windows

- System theme detection
- Windows 11 Mica/Acrylic effects
- Snap layouts support
- Taskbar integration

#### 10.3.3 Linux

- GTK/Qt theme detection
- System tray integration
- Desktop notification integration
- Wayland/X11 compatibility

### 10.4 Mobile Considerations

While WRAITH-Android and WRAITH-iOS have native implementations, design patterns should consider:

- Touch-friendly tap targets (minimum 44x44px)
- Swipe gestures for navigation
- Platform-specific navigation patterns (iOS bottom bar, Android material)
- Responsive layouts for tablets

---

## 11. Migration Guide

This section provides guidance for migrating existing clients to the standardized UI/UX patterns and for setting up new WRAITH clients.

### 11.1 Migrating from Tailwind v3 to v4

If your client uses the older Tailwind v3 pattern, migrate to v4 as follows:

**Before (Tailwind v3):**
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --wraith-primary: #7c3aed;
  --wraith-secondary: #4f46e5;
}
```

**After (Tailwind v4):**
```css
@import "tailwindcss";

@theme {
  --color-wraith-primary: #7c3aed;
  --color-wraith-secondary: #4f46e5;
}
```

**Key changes:**
1. Replace `@tailwind` directives with `@import "tailwindcss"`
2. Replace `:root` CSS variable declarations with `@theme` block
3. Prefix custom colors with `--color-` for Tailwind utility generation
4. Update any `tailwind.config.js` custom colors to use CSS variables

### 11.2 Standardization Checklist for New Clients

Use this checklist when creating a new WRAITH client application:

**CSS Setup:**
- [ ] Use `@import "tailwindcss"` (not `@tailwind` directives)
- [ ] Copy the complete `@theme` block from Section 8.1.3
- [ ] Include base styles for `body`, `html`, and `#root`
- [ ] Add `:focus-visible` styles for accessibility
- [ ] Add custom scrollbar styles

**Color Implementation:**
- [ ] Use standardized primary color (`#7c3aed`)
- [ ] Use standardized secondary color (`#4f46e5`)
- [ ] Use cyan accent (`#06b6d4`) or pink for chat apps (`#ec4899`)
- [ ] Use standardized background colors (`bg-primary`, `bg-secondary`, `bg-tertiary`)
- [ ] Use standardized text color hierarchy (`text-white`, `text-slate-200/300/400/500`)
- [ ] Use standardized border colors (`border-slate-700/600/500`)

**Component Patterns:**
- [ ] All buttons use standardized variants (Section 4.1.1)
- [ ] All modals use standardized pattern (Section 4.4.1)
- [ ] All inputs use standardized pattern (Section 4.2.1)
- [ ] Focus states use `focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-500`

**Accessibility:**
- [ ] All icon-only buttons have `aria-label`
- [ ] All modals have `role="dialog"`, `aria-modal="true"`, and `aria-labelledby`
- [ ] Form inputs have associated labels
- [ ] Error states include `aria-invalid`

### 11.3 Common Issues and Solutions

#### 11.3.1 TypeScript Import Type Errors

**Problem:** TypeScript complains about missing type exports.

**Solution:** Ensure types are exported correctly:
```tsx
// types/index.ts
export interface NodeStatus {
  running: boolean;
  node_id: string | null;
}

// Avoid: export type { NodeStatus } from './NodeStatus';
// Prefer: Re-export with explicit interface definition
```

#### 11.3.2 CSS Custom Properties Not Working

**Problem:** Tailwind utility classes like `bg-wraith-primary` don't apply.

**Solution:** Ensure CSS variable names in `@theme` are prefixed with `--color-`:
```css
@theme {
  /* Correct */
  --color-wraith-primary: #7c3aed;

  /* Incorrect - won't generate utilities */
  --wraith-primary: #7c3aed;
}
```

#### 11.3.3 Focus Ring Not Visible

**Problem:** Focus states are not visible on interactive elements.

**Solution:** Use `focus-visible` instead of `focus` and ensure the ring color has sufficient contrast:
```tsx
// Correct
className="focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-500"

// Incorrect - may not show on keyboard navigation
className="focus:ring-2 focus:ring-cyan-500"
```

#### 11.3.4 Modal Backdrop Click Not Working

**Problem:** Clicking the backdrop doesn't close the modal.

**Solution:** Ensure the click handler is on the outer element and propagation is stopped on the inner element:
```tsx
<div onClick={onClose}>  {/* Backdrop - handles click */}
  <div onClick={(e) => e.stopPropagation()}>  {/* Content - stops propagation */}
    {/* Modal content */}
  </div>
</div>
```

### 11.4 Testing Standardization

After standardizing a client, verify the following:

1. **Visual Consistency:** Compare side-by-side with WRAITH-Transfer
2. **Accessibility Audit:** Run browser accessibility tools (Chrome DevTools > Lighthouse)
3. **Keyboard Navigation:** Tab through all interactive elements
4. **Screen Reader Test:** Verify with VoiceOver (macOS) or NVDA (Windows)
5. **Focus Visibility:** Ensure all focusable elements have visible focus states

---

## Appendix A: Color Reference

### Full Color Palette

```css
/* Primary Scale */
--wraith-primary-50: #f5f3ff;
--wraith-primary-100: #ede9fe;
--wraith-primary-200: #ddd6fe;
--wraith-primary-300: #c4b5fd;
--wraith-primary-400: #a78bfa;
--wraith-primary-500: #8b5cf6;
--wraith-primary-600: #7c3aed; /* Primary */
--wraith-primary-700: #6d28d9;
--wraith-primary-800: #5b21b6;
--wraith-primary-900: #4c1d95;
--wraith-primary-950: #2e1065;

/* Accent Scale (Cyan) */
--wraith-accent-400: #22d3ee;
--wraith-accent-500: #06b6d4; /* Accent */
--wraith-accent-600: #0891b2;

/* Background Scale (Slate) */
--slate-700: #334155; /* Tertiary */
--slate-800: #1e293b; /* Secondary */
--slate-900: #0f172a; /* Primary */
--slate-950: #020617; /* Darker */
```

---

## Appendix B: Component Checklist

Use this checklist when implementing new components:

**Visual Standards:**
- [ ] Follows established color palette
- [ ] Uses standardized text color hierarchy (`text-white`, `text-slate-200/300/400/500`)
- [ ] Uses standardized border colors (`border-slate-700/600/500`)
- [ ] Uses standard spacing scale
- [ ] Has appropriate border radius

**Interaction States:**
- [ ] Includes hover state
- [ ] Has visible focus state (`focus-visible:ring-2 focus-visible:ring-cyan-500`)
- [ ] Loading state (if applicable)
- [ ] Error state (if applicable)
- [ ] Empty state (if applicable)

**Accessibility:**
- [ ] Supports keyboard navigation
- [ ] Has appropriate ARIA attributes
- [ ] Icon buttons have `aria-label`
- [ ] Modals have `role="dialog"`, `aria-modal="true"`, `aria-labelledby`
- [ ] Form inputs have associated labels and error descriptions

**Technical:**
- [ ] TypeScript props interface defined
- [ ] Responsive behavior defined
- [ ] Uses CSS variables from `@theme` block
- [ ] Documented in Storybook (future)

---

## Appendix C: References

### Internal Documentation

- `/home/parobek/Code/WRAITH-Protocol/clients/wraith-transfer/` - Reference implementation
- `/home/parobek/Code/WRAITH-Protocol/clients/wraith-chat/` - Chat patterns
- `/home/parobek/Code/WRAITH-Protocol/clients/wraith-sync/` - Sync patterns

### External Resources

- [Tailwind CSS Documentation](https://tailwindcss.com/docs)
- [Tauri 2.0 Documentation](https://v2.tauri.app/)
- [React Documentation](https://react.dev/)
- [Zustand Documentation](https://github.com/pmndrs/zustand)
- [WCAG 2.1 Guidelines](https://www.w3.org/TR/WCAG21/)

---

## Appendix D: Changelog

### v1.1.0 (2026-01-21)

**Major Updates:**
- Added Section 1.5 "Standardization Status" documenting current client standardization state
- Added standardized text color hierarchy (Section 3.1.4)
- Added standardized border color hierarchy (Section 3.1.5)
- Updated button patterns with verified working code (Section 4.1.1)
- Updated focus state pattern to use `focus-visible:ring-2 focus-visible:ring-cyan-500` (Section 4.1.2)
- Added icon button accessibility requirements (Section 4.1.4)
- Updated input field pattern with standardized classes (Section 4.2.1)
- Updated modal pattern with verified ARIA attributes and structure (Section 4.4.1)
- Added Tailwind v4 CSS architecture documentation (Section 8.1.2)
- Added complete CSS template (Section 8.1.3)
- Added Section 11 "Migration Guide" with:
  - Tailwind v3 to v4 migration instructions
  - Standardization checklist for new clients
  - Common issues and solutions
  - Testing standardization procedures
- Updated Appendix B component checklist with accessibility requirements

**Verification:**
- All patterns verified across WRAITH-Transfer, WRAITH-Chat, and WRAITH-Sync
- ARIA accessibility attributes tested and documented
- TypeScript compatibility verified

### v1.0.0 (2026-01-21)

- Initial release of UI/UX Design Reference
- Established core visual design system
- Documented component library patterns
- Defined accessibility standards (WCAG 2.1 AA)
- Created technical implementation guidelines

---

**Document Version:** 1.1.0
**Maintained By:** WRAITH Protocol Development Team
**Review Cycle:** Quarterly or with major feature releases
