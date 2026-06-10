// place files you want to import through the `$lib` alias in this folder.

import {
	PUBLIC_API_BASE,
	PUBLIC_FLUXER_MEDIA_PROXY_BASE,
	PUBLIC_FLUXER_STATIC_BASE,
} from '$env/static/public';
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

export function userProfilePictureUrl(
	user_id: string,
	avatar: string | null | undefined,
	size: number
): string {
	if (avatar) {
		return `${PUBLIC_FLUXER_MEDIA_PROXY_BASE}/avatars/${user_id}/${avatar}.webp?size=${size}`;
	} else {
		return `${PUBLIC_FLUXER_STATIC_BASE}/avatars/${BigInt(user_id) % 6n}.png`;
	}
}
