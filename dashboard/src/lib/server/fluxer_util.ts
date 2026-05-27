import z from 'zod';
import { GuildResponseSchema, type GuildResponse } from './fluxer_types';
import { FLUXER_API_BASE } from '$env/static/private';
import { getUserDataById } from './database';
import { error } from '@sveltejs/kit';

export async function getUserGuilds(user_id: string): Promise<GuildResponse[] | null> {
	const user_data = await getUserDataById(user_id);

	if (user_data === null) {
		return null;
	}

	const res = await fetch(`${FLUXER_API_BASE}/users/@me/guilds`, {
		headers: {
			Authorization: `Bearer ${user_data.server_data.access_token}`,
		},
	});

	const json = await res.json();

	if (!res.ok) {
		throw error(500, `Fluxer API returned error: ${res.status} with JSON: ${JSON.stringify(json)}`);
	}

	const user_guilds = z.array(GuildResponseSchema).parse(json);

	return user_guilds;
}
