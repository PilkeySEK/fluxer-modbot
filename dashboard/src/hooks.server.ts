import { getUserFromSession } from '$lib/server/session';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
	const sessionId = event.cookies.get('session_id');

	if (sessionId !== undefined) {
		const user = getUserFromSession(sessionId);
		event.locals.user = user;
	} else {
		event.locals.user = null;
	}

	return resolve(event);
};
