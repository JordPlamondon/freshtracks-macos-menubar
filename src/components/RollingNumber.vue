<template>
  <span class="rolling-number">
    <span v-for="(char, index) in currentChars" :key="`slot-${index}`" class="char-slot">
      <span v-if="char === '.' || char === ',' || char === '$'" class="static-char">
        {{ char }}
      </span>
      <span v-else class="digit-slot">
        <Transition name="roll">
          <span :key="charKeys[index]" class="digit">{{ char }}</span>
        </Transition>
      </span>
    </span>
  </span>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'

const props = defineProps<{
  value: number
  decimals?: number
  roundTo?: number
}>()

const decimals = props.decimals ?? 2
const roundTo = props.roundTo ?? 0.05

const currentChars = ref<string[]>([])
const charKeys = ref<string[]>([])
const previousValue = ref<number | null>(null)

const roundedValue = computed(() => {
  return Math.round(props.value / roundTo) * roundTo
})

const formatValue = (val: number): string[] => {
  const formatted = val.toLocaleString('en-US', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals
  })
  return `$${formatted}`.split('')
}

onMounted(() => {
  currentChars.value = formatValue(roundedValue.value)
  charKeys.value = currentChars.value.map((char, i) => `${i}-${char}-0`)
  previousValue.value = roundedValue.value
})

watch(roundedValue, (newValue) => {
  const newChars = formatValue(newValue)

  const newKeys = newChars.map((char, i) => {
    if (char !== currentChars.value[i] && char !== '.' && char !== ',' && char !== '$') {
      return `${i}-${char}-${Date.now()}`
    }
    return charKeys.value[i] || `${i}-${char}-0`
  })

  charKeys.value = newKeys
  currentChars.value = newChars
  previousValue.value = newValue
})
</script>

<style scoped>
.rolling-number {
  display: inline-flex;
  align-items: center;
  line-height: 1;
}

.char-slot {
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.static-char {
  display: inline-block;
  line-height: 1;
}

.digit-slot {
  display: inline-block;
  position: relative;
  overflow: hidden;
  width: 0.6em;
  text-align: center;
  line-height: 1;
}

.digit {
  display: block;
  width: 100%;
  text-align: center;
  line-height: 1;
}

/* Rolling animation */
.roll-enter-active {
  transition: transform 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

.roll-leave-active {
  transition: transform 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}

.roll-enter-from {
  transform: translateY(100%);
}

.roll-leave-to {
  transform: translateY(-100%);
}

.roll-leave-active {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
}
</style>
