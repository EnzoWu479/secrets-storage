<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{ checked?: boolean; modelValue?: boolean }>(),
  {
    checked: false,
    modelValue: undefined,
  },
);

const emit = defineEmits<{ "update:modelValue": [value: boolean] }>();

// Uso estático (:checked) ou interativo (v-model).
const on = computed(() => props.modelValue ?? props.checked);

function toggle() {
  emit("update:modelValue", !on.value);
}
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="on"
    class="inline-flex align-middle"
    @click="toggle"
  >
    <span
      class="ui-toggle__track inline-flex h-[22px] w-10 items-center rounded-pill p-0.5"
      :class="on ? 'bg-accent' : 'bg-divider'"
      aria-hidden="true"
    >
      <span
        class="ui-toggle__knob size-[18px] rounded-full bg-white transition-transform"
        :class="on ? 'translate-x-[18px]' : 'translate-x-0'"
      />
    </span>
  </button>
</template>
