import z from 'zod';
import { GuildResponseSchema, type GuildResponse } from './fluxer_types';
import { FLUXER_API_BASE } from '$env/static/private';
import { getUserDataById } from './database';

export async function getUserGuilds(user_id: string): Promise<GuildResponse[] | null> {
	const user_data = await getUserDataById(user_id);

	if (user_data === null) {
		return null;
	}

	const user_guilds = z.array(GuildResponseSchema).parse(
		await fetch(`${FLUXER_API_BASE}/users/@me/guilds`, {
			headers: {
				Authorization: `Bearer ${user_data.server_data.access_token}`,
			},
		}).then((res) => res.json())
	);

	return user_guilds;
}
