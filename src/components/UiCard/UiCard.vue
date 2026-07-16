<template>
  <article class="ui-card border border-divider bg-surface rounded-card p-5">
    <header v-if="$slots.header">
      <slot name="header" />
    </header>

    <slot />

    <footer v-if="$slots.footer">
      <slot name="footer" />
    </footer>
  </article>
</template>

<style scoped>
.ui-card {
  box-shadow: none;
}

:global(.theme-light) .ui-card {
  border-color: transparent;
  box-shadow: 0 4px 16px rgb(11 14 20 / 8%);
}
</style>
