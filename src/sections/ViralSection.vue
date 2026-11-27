<script setup lang="ts">
import { useParams } from "../composables/useParams";
import { VIRAL_PRESETS } from "../config/presets";
import ParamField from "../components/ParamField.vue";
import GroupEditor from "../components/GroupEditor.vue";
import PresetGroup from "../components/PresetGroup.vue";

const { params } = useParams();
</script>

<template>
  <PresetGroup
    :presets="VIRAL_PRESETS"
    hint="Sets the transmission and severity assumptions to values resembling a past influenza pandemic. Each value can be adjusted under Advanced settings, which switches the preset to Custom."
  >
    <ParamField path="r0" v-model="params.r0" />
    <ParamField path="latent_period" v-model="params.latent_period" />
    <ParamField path="infectious_period" v-model="params.infectious_period" />
    <GroupEditor path="p_symp_given_inf" v-model="params.p_symp_given_inf" />
    <GroupEditor path="p_hosp_given_symp" v-model="params.p_hosp_given_symp" />
    <GroupEditor
      path="p_death_given_hosp"
      v-model="params.p_death_given_hosp"
    />
  </PresetGroup>
</template>
