// first option
// export const bgColor = "#F7F7F7";
// export const primaryColor = "#2C3E50";
// export const secondaryColor = "#AED6F1";
// export const accentColor = "#82E0AA";
// export const darkGray = "#5D6D7E";
// export const mediumGray = "#D5D8DC";

import { writable } from "svelte/store";

const colorStore = writable({
  bgColor: "#F7F7F7",
  primaryColor: "#2C3E50",
  secondaryColor: "#AED6F1",
  accentColor: "#82E0AA",
  darkGray: "#5D6D7E",
  mediumGray: "#D5D8DC",
});

export default colorStore;
