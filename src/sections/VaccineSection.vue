<script setup lang="ts">
import { computed } from "vue";
import { NumberInput, SelectBox } from "cfasim-ui/components";
import MitigationSection from "../components/MitigationSection.vue";
import ParamField from "../components/ParamField.vue";
import { useParams } from "../composables/useParams";
import { getField } from "../config/uiConfig";

const { params } = useParams();

const dosesCfg = getField("vaccine_doses");
const dosesOptions = (dosesCfg.options ?? []).map((o) => ({
  value: String(o.value),
  label: o.label,
}));
const twoDose = computed(() => params.vaccine_doses === 2);
const dosesString = computed({
  get: () => String(params.vaccine_doses),
  set: (v: string) => {
    params.vaccine_doses = Number(v);
  },
});
const VE_SP = computed(
  () => 1.0 - (1.0 - params.vaccine_ve_s) * (1.0 - params.vaccine_ve_p)
);
</script>

<template>
  <MitigationSection
    label="Vaccine"
    :enabled="params.vaccine_enabled"
    @update:enabled="params.vaccine_enabled = $event"
  >
    <SelectBox
      :label="dosesCfg.label"
      v-model="dosesString"
      :options="dosesOptions"
    />
    <ParamField path="vaccine_start" v-model="params.vaccine_start" :max="params.days" />
    <ParamField
      path="vaccine_doses_available"
      v-model="params.vaccine_doses_available"
      :max="params.population"
    />
    <ParamField
      path="vaccine_administration_rate"
      v-model="params.vaccine_administration_rate"
    />
    <template v-if="params.vaccine_doses === 2">
      <ParamField
        path="vaccine_dose2_delay"
        v-model="params.vaccine_dose2_delay"
        :max="params.days"
      />
      <ParamField path="vaccine_p_get_2_doses" v-model="params.vaccine_p_get_2_doses" />
    </template>
    <!-- Two-dose mode folds each pair into one slider, a handle per dose. -->
    <ParamField
      path="vaccine_ve_s"
      v-model="params.vaccine_ve_s"
      :paired="twoDose"
      v-model:upper-value="params.vaccine_ve_2s"
    />
    <ParamField
      path="vaccine_ve_i"
      v-model="params.vaccine_ve_i"
      :paired="twoDose"
      v-model:upper-value="params.vaccine_ve_2i"
    />
    <ParamField
      path="vaccine_ve_p"
      v-model="params.vaccine_ve_p"
      :paired="twoDose"
      v-model:upper-value="params.vaccine_ve_2p"
    />
    <div class="ve-sp-readout" inert>
      <NumberInput
        label="Effectiveness against symptomatic disease"
        :model-value="VE_SP"
        :min="0"
        :max="1"
        :step="0.01"
        percent
        number-type="float"
        slider
      />
    </div>
    <ParamField path="vaccine_ramp_up" v-model="params.vaccine_ramp_up" />
  </MitigationSection>
</template>

<style scoped>
.ve-sp-readout {
  pointer-events: none;
}
</style>
