<script setup lang="ts">
import { computed, useId } from "vue";

const inputId = useId();

const props = withDefaults(
  defineProps<{
    label: string;
    type?: "text" | "password";
    help?: string;
    error?: string;
    placeholder?: string;
    value?: string;
    modelValue?: string;
  }>(),
  {
    type: "text",
    help: undefined,
    error: undefined,
    placeholder: undefined,
    value: undefined,
    modelValue: undefined,
  },
);

const emit = defineEmits<{ "update:modelValue": [value: string] }>();

// Suporta tanto uso estático (:value) quanto v-model (:modelValue).
const boundValue = computed(() => props.modelValue ?? props.value);

function onInput(event: Event) {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="grid gap-1.5">
    <label :for="inputId" class="text-sm font-medium text-primary">
      {{ label }}
    </label>

    <span class="relative">
      <input
        :id="inputId"
        :type="type"
        :placeholder="placeholder"
        :value="boundValue"
        :aria-invalid="error ? 'true' : undefined"
        class="h-10 w-full rounded-control border bg-surface px-3 text-sm text-primary outline-none placeholder:text-muted"
        :class="[
          error ? 'border-danger' : 'border-divider',
          type === 'password' ? 'pr-10' : undefined,
        ]"
        @input="onInput"
      />

      <span
        v-if="type === 'password'"
        class="ui-input__password-icon pointer-events-none absolute inset-y-0 right-3 flex items-center text-secondary"
        aria-hidden="true"
      >
        <slot name="password-icon">
          <svg
            class="size-4"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
          >
            <path d="M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6Z" />
            <circle cx="12" cy="12" r="2.5" />
          </svg>
        </slot>
      </span>
    </span>

    <span
      v-if="error || help"
      class="ui-input__message text-xs"
      :class="error ? 'text-danger' : 'text-secondary'"
    >
      {{ error || help }}
    </span>
  </div>
</template>
