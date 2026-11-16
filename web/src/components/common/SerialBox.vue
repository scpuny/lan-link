<script setup lang="ts">
import { ref } from 'vue'
import Icon from './Icon.vue'

const props = withDefaults(
  defineProps<{
    modelValue?: string
    editable?: boolean
    placeholder?: string
    error?: boolean
    /** 右下角灰标签，如「6 字符 / 组」 */
    hint?: string
    /** 错误态输入框右下角红标签，如「最后一组 5 字符」 */
    cornerTag?: string
    /** 框下方红色诊断说明 */
    errorText?: string
    /** 只读态标题行说明，如「未加密 / 未压缩 / 84 字符 · 14 组」 */
    meta?: string
  }>(),
  { editable: false, placeholder: '粘贴序列号，自动清洗大小写 / 空格 / 换行', error: false, hint: '6 字符 / 组' }
)
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>()

const copied = ref(false)
async function copy() {
  try {
    await navigator.clipboard.writeText(props.modelValue || '')
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
  } catch { /* ignore */ }
}
</script>

<template>
  <div class="serial-wrap">
    <div v-if="meta" class="sw-meta ll-text-tertiary">{{ meta }}</div>
    <div class="serial-box" :class="{ 'is-edit': editable, 'is-error': error, 'is-readonly': !editable }">
      <textarea
        v-if="editable"
        class="serial-input ll-mono"
        :value="modelValue"
        :placeholder="placeholder"
        spellcheck="false"
        @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
      />
      <template v-else>
        <div class="serial-display ll-mono">{{ modelValue || '尚未生成' }}</div>
        <button class="ll-btn ll-btn--text ll-btn--sm copy-btn" @click="copy">
          <Icon :name="copied ? 'check' : 'copy'" :size="12" /> {{ copied ? '已复制' : '复制' }}
        </button>
      </template>

      <span v-if="editable && cornerTag" class="sw-corner-tag">{{ cornerTag }}</span>
      <span v-else-if="editable && hint" class="sw-hint">{{ hint }}</span>
      <span v-if="editable && error" class="sw-error-icon"><Icon name="alertCircle" :size="14" /></span>
    </div>
    <p v-if="errorText" class="sw-error-text">{{ errorText }}</p>
  </div>
</template>

<style scoped>
.serial-wrap { position: relative; }
.sw-meta { font-size: var(--ll-fs-xs); margin-bottom: var(--ll-space-2); }
.serial-box {
  position: relative;
  display: flex;
  align-items: center;
  background: var(--ll-color-surface);
  border: 1px solid var(--ll-color-border-strong);
  border-radius: var(--ll-radius-sm);
  padding: var(--ll-space-2) var(--ll-space-3);
}
.serial-box.is-error { border-color: var(--ll-color-danger); }
.serial-box.is-edit { align-items: stretch; padding: 0; min-height: 62px; }
.serial-box.is-readonly { background: var(--ll-serial-bg); border-color: var(--ll-color-border); }
.serial-input {
  flex: 1;
  width: 100%;
  min-height: 62px;
  resize: none;
  border: none;
  background: transparent;
  color: var(--ll-color-serial-text);
  font-size: var(--ll-fs-md);
  line-height: var(--ll-lh-relaxed);
  letter-spacing: 0.5px;
  padding: var(--ll-space-3);
  padding-right: 84px;
  outline: none;
}
.serial-input::placeholder { color: var(--ll-color-text-quaternary); }
.serial-box.is-edit:focus-within { border-color: var(--ll-color-brand); box-shadow: 0 0 0 3px var(--ll-color-brand-bg); }
.serial-display {
  flex: 1;
  font-size: var(--ll-fs-md);
  color: var(--ll-color-serial-text);
  overflow-x: auto;
  white-space: nowrap;
  padding: var(--ll-space-1) 0;
}
.copy-btn { flex: none; }
.sw-hint, .sw-corner-tag {
  position: absolute;
  right: var(--ll-space-2);
  bottom: var(--ll-space-2);
  font-size: 10px;
  color: var(--ll-color-text-tertiary);
  background: var(--ll-color-surface-3);
  border-radius: var(--ll-radius-xs);
  padding: 1px 5px;
}
.sw-corner-tag { color: var(--ll-color-danger); background: var(--ll-color-danger-light); }
.sw-error-icon { position: absolute; right: var(--ll-space-2); top: var(--ll-space-3); color: var(--ll-color-danger); display: inline-flex; }
.sw-error-text { margin-top: var(--ll-space-2); font-size: var(--ll-fs-xs); color: var(--ll-color-danger); }
</style>
