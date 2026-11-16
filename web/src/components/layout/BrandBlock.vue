<script setup lang="ts">
// 品牌标识块：图标 + （名称 / 描述）两行竖排
// 目前只用于命令栏左侧；抽成组件是为了标题栏（缩放/窄窗）等场景可复用同一视觉。
import Icon from '../common/Icon.vue'

withDefaults(defineProps<{ iconSize?: number; sub?: boolean }>(), {
  iconSize: 16,
  sub: true,
})
</script>

<template>
  <div class="brand" data-tauri-drag-region>
    <span class="brand-logo">
      <Icon name="link" :size="iconSize" :width="2.2" />
    </span>
    <span class="brand-text">
      <span class="brand-name">LanLink</span>
      <span v-if="sub" class="brand-sub">局域网 / 跨网 P2P 文件传输</span>
    </span>
  </div>
</template>

<style scoped>
.brand {
  display: flex;
  align-items: center;
  gap: var(--ll-space-2);
  min-width: 0;
  flex: none;
  user-select: none;
}
.brand-logo {
  width: 26px;
  height: 26px;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--ll-radius-sm);
  background: var(--ll-color-brand);
  color: var(--ll-color-text-inverse);
}
/* 名称与描述上下两行 */
.brand-text {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 1px;
  min-width: 0;
  line-height: 1.15;
}
.brand-name {
  font-size: var(--ll-fs-lg);
  font-weight: var(--ll-fw-semibold);
  color: var(--ll-color-text);
  letter-spacing: 0.1px;
}
.brand-sub {
  font-size: var(--ll-fs-xs);
  color: var(--ll-color-text-tertiary);
  white-space: nowrap;
}
</style>
