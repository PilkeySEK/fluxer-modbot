import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { getUserGuilds } from '$lib/server/fluxer_util';
import type { GuildResponse } from '$lib/server/fluxer_types';

export const GET: RequestHandler = async ({ params, locals }) => {
	let user_id;

	if (params.user_id === '@me') {
		if (locals.user === null) {
			throw error(401, 'Not logged in');
		}

		user_id = locals.user.id;
	} else {
		if (!locals.user?.is_bot_admin) {
			throw error(403, 'You are not bot staff');
		}

		user_id = params.user_id;
	}

	let guilds: GuildResponse[] | null;
	try {
		guilds = await getUserGuilds(user_id);
	} catch (e) {
		console.error(e);
		throw error(500);
	}

	if (guilds === null) {
		throw error(401);
	}

	return new Response(JSON.stringify(guilds));
};
