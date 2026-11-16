<script setup lang="ts">
import { computed } from 'vue'
import Icon from './Icon.vue'

const props = withDefaults(
  defineProps<{
    title: string
    icon?: string
    badge?: string
    badgeVariant?: 'brand' | 'success' | 'warning' | 'danger' | 'info' | ''
    width?: number
    closable?: boolean
    /** 原型对话框无副标题，仅少数页面需要 */
    subtitle?: string
  }>(),
  { badgeVariant: '', width: 520, closable: true }
)
const emit = defineEmits<{ (e: 'close'): void }>()

const badgeClass = computed(() =>
  props.badgeVariant ? `ll-tag ll-tag--${props.badgeVariant}` : 'll-tag'
)
</script>

<template>
  <Teleport to="body">
    <div class="ll-modal-mask" @click.self="closable && emit('close')">
      <div class="ll-modal" :style="{ width: width + 'px' }" role="dialog" aria-modal="true">
        <header class="ll-modal__head">
          <div class="mh-left">
            <span v-if="icon" class="mh-icon"><Icon :name="icon" :size="16" /></span>
            <h3 class="ll-modal__title ll-truncate">{{ title }}</h3>
          </div>
          <div class="mh-right">
            <slot name="header-extra" />
            <span v-if="badge" :class="badgeClass">{{ badge }}</span>
            <button v-if="closable" class="mh-close" aria-label="关闭" @click="emit('close')">
              <Icon name="close" :size="15" />
            </button>
          </div>
        </header>
        <p v-if="subtitle" class="ll-modal__sub">{{ subtitle }}</p>
        <div class="ll-modal__body"><slot /></div>
        <footer v-if="$slots.footer" class="ll-modal__foot"><slot name="footer" /></footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.ll-modal-mask {
  position: fixed;
  inset: 0;
  z-index: var(--ll-z-dialog);
  background: var(--ll-color-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--ll-space-6);
  animation: ll-fade var(--ll-duration-base) var(--ll-ease-standard);
}
.ll-modal {
  max-width: calc(100vw - 48px);
  max-height: calc(100vh - 48px);
  display: flex;
  flex-direction: column;
  background: var(--ll-color-surface);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-lg);
  box-shadow: var(--ll-shadow-pop);
  animation: ll-pop var(--ll-duration-base) var(--ll-ease-out);
}
.ll-modal__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ll-space-3);
  padding: var(--ll-space-4) var(--ll-space-5);
  border-bottom: 1px solid var(--ll-color-divider);
}
.mh-left { display: flex; align-items: center; gap: var(--ll-space-2); min-width: 0; }
.mh-icon { color: var(--ll-color-brand); display: inline-flex; flex: none; }
.ll-modal__title { font-size: var(--ll-fs-lg); font-weight: var(--ll-fw-semibold); }
.mh-right { display: flex; align-items: center; gap: var(--ll-space-3); flex: none; }
.mh-close {
  width: 24px; height: 24px; border: none; background: transparent;
  color: var(--ll-color-text-tertiary); border-radius: var(--ll-radius-xs);
  display: inline-flex; align-items: center; justify-content: center;
}
.mh-close:hover { background: var(--ll-color-surface-hover); color: var(--ll-color-text); }
.ll-modal__sub { font-size: var(--ll-fs-sm); color: var(--ll-color-text-tertiary); padding: 0 var(--ll-space-5); margin-top: var(--ll-space-2); }
.ll-modal__body { padding: var(--ll-space-4) var(--ll-space-5); overflow: auto; }
.ll-modal__foot {
  padding: var(--ll-space-3) var(--ll-space-5) var(--ll-space-4);
  border-top: 1px solid var(--ll-color-divider);
}
@keyframes ll-fade { from { opacity: 0; } to { opacity: 1; } }
@keyframes ll-pop { from { opacity: 0; transform: translateY(8px) scale(0.98); } to { opacity: 1; transform: none; } }
</style>
