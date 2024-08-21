const defaultTheme = require('tailwindcss/defaultTheme');

const config = {
	content: [
		'./src/**/*.{html,js,svelte,ts,md}',
		'./node_modules/flowbite-svelte/**/*.{html,js,svelte,ts}'
	],
	plugins: [require('flowbite/plugin')],
	darkMode: 'class',
	theme: {
		fontFamily: {
			sans: ['Inter', ...defaultTheme.fontFamily.sans]
		},
		extend: {
			colors: {
				primary: {
					DEFAULT: '#177FBF', // Blue
					50: '#e5f3fa', // lighter blue
					100: '#cce8f6', // light blue
					200: '#99d1ed', // medium light blue
					300: '#66b9e3', // medium blue
					400: '#339ad8', // normal blue
					500: '#177FBF', // your specific blue
					600: '#106399', // dark blue
					700: '#0a4773', // darker blue
					800: '#05304d', // very dark blue
					900: '#021827' // deepest blue
				},
				secondary: {
					indigo: {
						DEFAULT: '#162D73', // Indigo
						50: '#e3e5ec', // lighter indigo
						100: '#c6cae0', // light indigo
						200: '#9d9fcd', // medium light indigo
						300: '#7375ba', // medium indigo
						400: '#4a4ba6', // normal indigo
						500: '#162D73', // your specific indigo
						600: '#121e5a', // dark indigo
						700: '#0e1740', // darker indigo
						800: '#0b1027', // very dark indigo
						900: '#060813' // deepest indigo
					},
					green: {
						DEFAULT: '#41D9AE', // Green
						50: '#e6faf4', // lighter green
						100: '#ccf5e9', // light green
						200: '#99ebd2', // medium light green
						300: '#66e1bc', // medium green
						400: '#33d7a5', // normal green
						500: '#41D9AE', // your specific green
						600: '#2ca882', // dark green
						700: '#21785d', // darker green
						800: '#154839', // very dark green
						900: '#0b2418' // deepest green
					},
					grey: {
						DEFAULT: '#404040', // Grey
						50: '#f0f0f0', // lighter grey
						100: '#d9d9d9', // light grey
						200: '#b3b3b3', // medium light grey
						300: '#8d8d8d', // medium grey
						400: '#666666', // normal grey
						500: '#404040', // your specific grey
						600: '#303030', // dark grey
						700: '#202020', // darker grey
						800: '#101010', // very dark grey
						900: '#000000' // deepest grey
					},
					black: '#000000' // Black
				}
			}
		}
	}
};

module.exports = config;
