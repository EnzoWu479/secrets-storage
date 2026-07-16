<script setup lang="ts">
type StrengthLevel = 1 | 2 | 3 | 4;

defineProps<{
  level: StrengthLevel;
}>();

const strengths: Record<StrengthLevel, { label: string; color: string }> = {
  1: { label: "Fraca", color: "bg-danger" },
  2: { label: "Média", color: "bg-warning" },
  3: { label: "Boa", color: "bg-accent" },
  4: { label: "Forte", color: "bg-success" },
};
</script>

<template>
  <div
    role="meter"
    aria-label="Força da senha"
    aria-valuemin="1"
    aria-valuemax="4"
    :aria-valuenow="level"
    :aria-valuetext="strengths[level].label"
  >
    <div class="flex gap-1" aria-hidden="true">
      <span
        v-for="segment in 4"
        :key="segment"
        data-strength-segment
        class="h-1 flex-1 rounded-pill"
        :class="segment <= level ? strengths[level].color : 'bg-divider'"
      />
    </div>

    <span data-strength-label class="mt-2 block text-sm text-secondary">
      {{ strengths[level].label }}
    </span>
  </div>
</template>
