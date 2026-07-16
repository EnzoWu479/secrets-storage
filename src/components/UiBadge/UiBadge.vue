<script setup lang="ts">
type BadgeTone = "neutral" | "accent" | "success" | "warning" | "danger";

const props = withDefaults(
  defineProps<{
    tone?: BadgeTone;
  }>(),
  {
    tone: "neutral",
  },
);

const toneClasses: Record<BadgeTone, string[]> = {
  neutral: ["bg-divider", "text-secondary"],
  accent: ["bg-[var(--color-accent-soft)]", "text-accent"],
  success: ["bg-success/10", "text-success"],
  warning: ["bg-warning/10", "text-warning"],
  danger: ["bg-danger/10", "text-danger"],
};
</script>

<template>
  <span
    :class="[
      'inline-flex items-center rounded-pill px-2 py-0.5 text-[11px] font-medium leading-[14px]',
      toneClasses[props.tone],
    ]"
  >
    <slot />
  </span>
</template>
