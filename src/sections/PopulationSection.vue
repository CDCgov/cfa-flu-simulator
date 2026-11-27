<script setup lang="ts">
import { useParams } from "../composables/useParams";
import { POPULATION_PRESETS } from "../config/presets";
import ParamField from "../components/ParamField.vue";
import PresetGroup from "../components/PresetGroup.vue";

const { params } = useParams();
</script>

<template>
  <PresetGroup
    :presets="POPULATION_PRESETS"
    hint="Sets the population size, initial infections and prior immunity. Each value can be adjusted under Advanced settings, which switches the preset to Custom."
  >
    <ParamField path="population" v-model="params.population" />
    <ParamField path="initial_infections" v-model="params.initial_infections" />
    <ParamField
      path="fraction_initial_immune"
      v-model="params.fraction_initial_immune"
    />
  </PresetGroup>
</template>
