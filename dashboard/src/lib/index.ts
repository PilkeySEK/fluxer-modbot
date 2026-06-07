// place files you want to import through the `$lib` alias in this folder.

import { PUBLIC_API_BASE } from '$env/static/public';
import type { Snippet } from 'svelte';
import type { paths } from './api';
import createClient from 'openapi-fetch';
import type { HTMLButtonAttributes } from 'svelte/elements';

// Tailwind var(--spacing) is 0.25rem
const SPACING = 0.25;

export const api = createClient<paths>({ baseUrl: PUBLIC_API_BASE });

export type ApiRes<P extends keyof paths> = paths[P] extends {
	get: {
		responses: {
			200: {
				content: {
					'application/json': infer R;
				};
			};
		};
	};
}
	? R
	: never;

export function iconSize(size: number): string {
	return `${size * SPACING}rem`;
}

export type GenericButtonProps = {
	children: Snippet;
	retain_size?: boolean;
} & HTMLButtonAttributes;
