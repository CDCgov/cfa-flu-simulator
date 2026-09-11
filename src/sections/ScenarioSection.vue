<script setup lang="ts">
import { useParams } from "../composables/useParams";
import ParamField from "../components/ParamField.vue";
import GroupEditor from "../components/GroupEditor.vue";

const { params } = useParams();
</script>

<template>
  <div class="section">
    <ParamField path="days" v-model="params.days" />
    <ParamField path="population" v-model="params.population" />
    <ParamField path="initial_infections" v-model="params.initial_infections" />
    <ParamField
      path="fraction_initial_immune"
      v-model="params.fraction_initial_immune"
    />
    <ParamField path="r0" v-model="params.r0" />
    <ParamField path="latent_period" v-model="params.latent_period" />
    <ParamField path="infectious_period" v-model="params.infectious_period" />
    <GroupEditor path="p_symp_given_inf" v-model="params.p_symp_given_inf" />
    <GroupEditor path="p_hosp_given_symp" v-model="params.p_hosp_given_symp" />
    <GroupEditor
      path="p_death_given_hosp"
      v-model="params.p_death_given_hosp"
    />
  </div>
</template>

<style scoped>
.section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
</style>
