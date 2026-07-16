<script setup lang="ts">
type IconName =
  | "lock"
  | "vault"
  | "eye"
  | "copy"
  | "google"
  | "google-drive"
  | "password"
  | "api"
  | "token"
  | "note"
  | "ssh"
  | "settings"
  | "sync"
  | "warning";

const props = withDefaults(
  defineProps<{
    name: IconName;
    size?: number | string;
  }>(),
  {
    size: 20,
  },
);

const iconPaths: Record<IconName, string[]> = {
  lock: [
    "M7 10V7a5 5 0 0 1 10 0v3",
    "M5 10h14v10H5z M12 14v2",
  ],
  vault: [
    "M3 5h18v14H3z",
    "M7 9h9v6H7z M11.5 9v6 M7 12h9 M18 10v4",
  ],
  eye: [
    "M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6S2.5 12 2.5 12Z",
    "M9 12a3 3 0 1 0 6 0 3 3 0 0 0-6 0Z",
  ],
  copy: [
    "M8 8h11v11H8z",
    "M16 8V5H5v11h3",
  ],
  google: [
    "M20 12.2c0-.7-.1-1.4-.2-2H12v3.5h4.5a4.7 4.7 0 0 1-4.5 3.2 5 5 0 1 1 0-9.8c1.3 0 2.4.4 3.3 1.3l2.5-2.5A8.5 8.5 0 1 0 20 12.2Z",
  ],
  "google-drive": [
    "M9 3h6l6 10-3 5H6l-3-5L9 3Z",
    "M9 3l6 10h6 M15 13l-3 5 M3 13h12",
  ],
  password: [
    "M14 8a4 4 0 1 0-3.5 4H21v4h-3v3h-4v-7",
    "M6 8h.01",
  ],
  api: [
    "M8 5H5v14h3 M16 5h3v14h-3",
    "M9 9h6 M9 15h6 M12 9v6",
  ],
  token: [
    "M12 2.5 20 7v10l-8 4.5L4 17V7l8-4.5Z",
    "M9 12a3 3 0 1 0 6 0 3 3 0 0 0-6 0Z",
  ],
  note: [
    "M6 3h9l4 4v14H6z M15 3v5h4",
    "M9 12h6 M9 16h6",
  ],
  ssh: [
    "M4 5h16v14H4z",
    "m8 9 3 3-3 3 M13 15h3",
  ],
  settings: [
    "M12 2.5v3 M12 18.5v3 M2.5 12h3 M18.5 12h3 M5.3 5.3l2.1 2.1 M16.6 16.6l2.1 2.1 M18.7 5.3l-2.1 2.1 M7.4 16.6l-2.1 2.1",
    "M8 12a4 4 0 1 0 8 0 4 4 0 0 0-8 0Z",
  ],
  sync: [
    "M20 7v5h-5 M4 17v-5h5",
    "M18.5 10A7 7 0 0 0 6.7 6.7L4 9 M5.5 14A7 7 0 0 0 17.3 17.3L20 15",
  ],
  warning: [
    "M12 3 22 20H2L12 3Z",
    "M12 9v5 M12 17h.01",
  ],
};
</script>

<template>
  <svg
    :data-icon="props.name"
    :width="props.size"
    :height="props.size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    <path
      v-for="(path, index) in iconPaths[props.name]"
      :key="index"
      :d="path"
    />
  </svg>
</template>
