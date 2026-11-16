<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'

export interface MenuItem {
  label: string
  key?: string
  hint?: string
  checked?: boolean
  sep?: boolean
  submenu?: string[]
}
export interface MenuGroup {
  label: string
  items: MenuItem[]
}

defineProps<{ menus: MenuGroup[] }>()
const emit = defineEmits<{ (e: 'action', payload: { key: string; value?: string }): void }>()

const openIndex = ref(-1)
const subOpen = ref(-1)

function toggle(i: number) {
  openIndex.value = openIndex.value === i ? -1 : i
  subOpen.value = -1
}
function hoverTop(i: number) {
  if (openIndex.value !== -1) {
    openIndex.value = i
    subOpen.value = -1
  }
}
function close() {
  openIndex.value = -1
  subOpen.value = -1
}
function pick(it: MenuItem, value?: string) {
  if (!it.key) return
  emit('action', { key: it.key, value })
  close()
}

function onDocClick(e: MouseEvent) {
  const el = e.target as HTMLElement
  if (!el.closest('.app-menu')) close()
}
onMounted(() => document.addEventListener('click', onDocClick))
onBeforeUnmount(() => document.removeEventListener('click', onDocClick))
</script>

<template>
  <div class="app-menu" @click.stop>
    <div v-for="(m, i) in menus" :key="i" class="am-top">
      <button class="am-top-btn" :class="{ active: openIndex === i }" @click="toggle(i)" @mouseenter="hoverTop(i)">
        {{ m.label }}
      </button>
      <div v-if="openIndex === i" class="am-dropdown">
        <template v-for="(it, j) in m.items" :key="j">
          <div v-if="it.sep" class="am-sep" />
          <div
            v-else
            class="am-item"
            :class="{ 'has-sub': it.submenu }"
            @mouseenter="subOpen = it.submenu ? j : -1"
            @click="pick(it)"
          >
            <span class="am-label">{{ it.label }}</span>
            <span v-if="it.hint" class="am-hint">{{ it.hint }}</span>
            <span v-else-if="it.checked" class="am-check">✓</span>
            <span v-if="it.submenu" class="am-arrow">›</span>
            <div v-if="it.submenu && subOpen === j" class="am-sub">
              <div v-for="s in it.submenu" :key="s" class="am-item" @click.stop="pick(it, s)">
                {{ s }}
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app-menu { display: flex; align-items: stretch; height: 100%; }
.am-top { position: relative; display: flex; align-items: stretch; }
.am-top-btn {
  border: none;
  background: transparent;
  color: var(--ll-color-text);
  font-size: var(--ll-fs-sm);
  padding: 0 var(--ll-space-3);
  border-radius: var(--ll-radius-xs);
  transition: background var(--ll-duration-fast);
}
.am-top-btn:hover, .am-top-btn.active { background: var(--ll-color-surface-hover); }
.am-dropdown {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  min-width: 200px;
  background: var(--ll-color-surface);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  box-shadow: var(--ll-shadow-3);
  padding: var(--ll-space-1);
  z-index: var(--ll-z-menu);
  animation: ll-fade var(--ll-duration-fast) var(--ll-ease-standard);
}
.am-item {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ll-space-4);
  padding: 7px var(--ll-space-3);
  font-size: var(--ll-fs-sm);
  color: var(--ll-color-text);
  border-radius: var(--ll-radius-xs);
  cursor: pointer;
}
.am-item:hover { background: var(--ll-color-brand-bg); color: var(--ll-color-brand); }
.am-hint { color: var(--ll-color-text-tertiary); font-size: var(--ll-fs-xs); font-family: var(--ll-font-mono); }
.am-check { color: var(--ll-color-brand); }
.am-arrow { color: var(--ll-color-text-tertiary); }
.am-sep { height: 1px; background: var(--ll-color-divider); margin: var(--ll-space-1) 0; }
.am-sub {
  position: absolute;
  top: -1px;
  left: calc(100% + 4px);
  min-width: 140px;
  background: var(--ll-color-surface);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  box-shadow: var(--ll-shadow-3);
  padding: var(--ll-space-1);
}
</style>
