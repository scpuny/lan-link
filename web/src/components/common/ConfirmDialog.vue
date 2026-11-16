<script setup lang="ts">
/** 危险操作统一二次确认弹窗：由 useUi 的 confirm() Promise 驱动 */
import { useUi } from '../../composables/useUi'
import Modal from './Modal.vue'
import Icon from './Icon.vue'

const { confirmRequest, resolveConfirm } = useUi()
</script>

<template>
  <Modal
    v-if="confirmRequest"
    :title="confirmRequest.title"
    icon="alertTriangle"
    :badge="confirmRequest.tone === 'danger' ? '不可撤销' : ''"
    badge-variant="danger"
    :width="440"
    @close="resolveConfirm(false)"
  >
    <div class="confirm-body">
      <p class="confirm-message">{{ confirmRequest.message }}</p>
    </div>
    <template #footer>
      <div class="confirm-foot">
        <button class="ll-btn ll-btn--default" @click="resolveConfirm(false)">
          {{ confirmRequest.cancelText }}
        </button>
        <button
          class="ll-btn"
          :class="confirmRequest.tone === 'danger' ? 'll-btn--danger' : 'll-btn--primary'"
          @click="resolveConfirm(true)"
        >
          {{ confirmRequest.confirmText }}
        </button>
      </div>
    </template>
  </Modal>
</template>

<style scoped>
.confirm-body { padding: var(--ll-space-1) 0 var(--ll-space-2); }
.confirm-message {
  font-size: var(--ll-fs-sm);
  line-height: 1.7;
  color: var(--ll-color-text-secondary);
  white-space: pre-line;
}
.confirm-foot { display: flex; justify-content: flex-end; gap: var(--ll-space-2); }
</style>
