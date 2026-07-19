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

export type ApiRes<P extends keyof paths, M extends keyof paths[P] = 'get'> = paths[P][M] extends {
	responses: {
		200: {
			content: {
				'application/json': infer R;
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

/**
 * Format a duration into a human-readable format like "1d 3h 5m 52s"
 */
export function prettyDuration(duration: { secs: number; nanos: number }): string {
	const totalSeconds = Math.floor(duration.secs + duration.nanos / 1000000000);
	const seconds = totalSeconds % 60;
	const minutes = Math.floor(totalSeconds / 60) % 60;
	const hours = Math.floor(totalSeconds / 60 / 60) % 24;
	const days = Math.floor(totalSeconds / 60 / 60 / 24);

	let s = '';

	if (days > 0) {
		s += `${days}d `;
	}
	if (hours > 0 || days > 0) {
		s += `${hours}h `;
	}
	if (minutes > 0 || hours > 0 || days > 0) {
		s += `${minutes}m `;
	}
	if (seconds > 0 || minutes > 0 || hours > 0 || days > 0) {
		s += `${seconds}s `;
	}

	return s.trim();
}
