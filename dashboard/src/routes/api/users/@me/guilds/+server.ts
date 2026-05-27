import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getUserGuilds } from '$lib/server/fluxer_util';
import type { GuildResponse } from '$lib/server/fluxer_types';

export const GET: RequestHandler = async ({ locals }) => {
	if (locals.user === null) {
		throw error(401, 'Not logged in');
	}

	let guilds: GuildResponse[] | null;
	try {
		guilds = await getUserGuilds(locals.user.id);
	} catch (e) {
		console.error(e);
		throw error(500, 'Failed to get guilds');
	}

	if (guilds === null) {
		throw error(401, 'No active session');
	}

	return new Response(JSON.stringify(guilds));
};
