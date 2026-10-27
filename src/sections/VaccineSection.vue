<script setup lang="ts">
import { computed } from "vue";
import { Hint, SelectBox } from "cfasim-ui/components";
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

// VE_SP = 1 - (1 - VE_S)(1 - VE_P): overall protection against symptomatic
// disease, combining protection from infection with protection from symptoms
// given infection. A readout per dose, since the editable fields determine it.
const overall = computed(() => {
  const doses = [
    {
      label: "1st dose",
      value: 1 - (1 - params.vaccine_ve_s) * (1 - params.vaccine_ve_p),
    },
  ];
  if (twoDose.value) {
    doses.push({
      label: "2nd dose",
      value: 1 - (1 - params.vaccine_ve_2s) * (1 - params.vaccine_ve_2p),
    });
  }
  return doses;
});
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
    <template v-if="twoDose">
      <ParamField
        path="vaccine_dose2_delay"
        v-model="params.vaccine_dose2_delay"
        :max="params.days"
        :disabled-ranges="[[0, params.vaccine_ramp_up]]"
      />
      <ParamField path="vaccine_p_get_2_doses" v-model="params.vaccine_p_get_2_doses" />
    </template>
    <ParamField path="vaccine_ramp_up" v-model="params.vaccine_ramp_up" />

    <!-- Two-dose mode folds each pair into one slider, a handle per dose. -->
    <div class="ve">
      <span class="ve__title">Effectiveness</span>
      <div class="ve__derived">
        <span class="ve__derived-label">
          Effectiveness against symptomatic disease
          <Hint
            text="Total protection against symptomatic disease (not conditioned on infection)"
          />
        </span>
        <output class="ve__derived-value">
          <span v-for="dose in overall" :key="dose.label">
            <span v-if="twoDose" class="ve__dose-label">{{ dose.label }}</span>
            {{ Math.round(dose.value * 100) }}%
          </span>
        </output>
      </div>
      <ParamField
        path="vaccine_ve_s"
        v-model="params.vaccine_ve_s"
        :paired="twoDose"
        v-model:upper-value="params.vaccine_ve_2s"
      />
      <ParamField
        path="vaccine_ve_p"
        v-model="params.vaccine_ve_p"
        :paired="twoDose"
        v-model:upper-value="params.vaccine_ve_2p"
      />
    </div>
  </MitigationSection>
</template>

<style scoped>
.ve {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding-top: var(--space-3);
  border-top: 1px solid var(--color-border);
}
.ve__title {
  font-weight: 600;
  font-size: var(--font-size-sm);
}
.ve__derived {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-2);
  font-size: var(--font-size-sm);
}
.ve__derived-label {
  display: flex;
  align-items: center;
  gap: 0.25em;
}
.ve__derived-value {
  display: flex;
  gap: var(--space-4);
  font-weight: 600;
  line-height: 1.2;
}
.ve__derived-value > span {
  display: grid;
}
.ve__dose-label {
  font-size: var(--font-size-xs);
  font-weight: 400;
  color: var(--color-text-secondary);
}
</style>
