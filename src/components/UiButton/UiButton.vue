<script setup lang="ts">
type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";

const props = withDefaults(
  defineProps<{
    variant?: ButtonVariant;
  }>(),
  {
    variant: "primary",
  },
);

const commonClasses = [
  "inline-flex",
  "items-center",
  "justify-center",
  "rounded-control",
  "font-medium",
  "transition-colors",
  "focus-visible:outline-none",
  "focus-visible:ring-2",
  "focus-visible:ring-accent",
  "focus-visible:ring-offset-2",
  "focus-visible:ring-offset-app",
  "disabled:cursor-not-allowed",
  "disabled:bg-muted",
  "disabled:text-secondary",
];

const variantClasses: Record<ButtonVariant, string[]> = {
  primary: ["h-10", "px-4", "bg-accent", "text-white", "hover:bg-accent-hover"],
  secondary: [
    "h-10",
    "border",
    "border-divider",
    "bg-transparent",
    "px-4",
    "text-primary",
    "hover:bg-elevated",
  ],
  danger: ["h-10", "px-4", "bg-danger", "text-white", "hover:opacity-90"],
  ghost: ["h-9", "w-9", "bg-transparent", "text-secondary", "hover:bg-elevated", "hover:text-primary"],
};
</script>

<template>
  <button :class="[commonClasses, variantClasses[props.variant]]">
    <slot />
  </button>
</template>
