import nextCoreWebVitals from 'eslint-config-next/core-web-vitals';

/** @type {import('eslint').Linter.FlatConfig[]} */
const config = [
	{
		ignores: ['.next/**', '.intlayer/**', 'out/**'],
	},
	...nextCoreWebVitals,
	{
		rules: {
			'react-hooks/set-state-in-effect': 'off',
			'react-hooks/preserve-manual-memoization': 'off',
		},
	},
];

export default config;
