<script setup lang="ts">
import { computed } from "vue";
import MitigationSection from "../components/MitigationSection.vue";
import ParamField from "../components/ParamField.vue";
import MatrixInput from "../components/MatrixInput.vue";
import { useParams } from "../composables/useParams";

const { params } = useParams();
const durationMax = computed(() =>
  Math.max(1, params.days - params.community_start),
);
</script>

<template>
  <MitigationSection
    label="Contact-pattern interventions"
    :enabled="params.community_enabled"
    @update:enabled="params.community_enabled = $event"
  >
    <ParamField
      path="community_start"
      v-model="params.community_start"
      :max="params.days"
    />
    <ParamField
      path="community_duration"
      v-model="params.community_duration"
      :max="durationMax"
    />
    <MatrixInput
      path="community_effectiveness"
      v-model="params.community_effectiveness"
    />
  </MitigationSection>
</template>
