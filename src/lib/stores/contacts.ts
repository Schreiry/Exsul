import { writable } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type {
	Contact,
	ContactLocation,
	CreateContactLocationPayload,
	CreateContactPayload,
	Order,
	UpdateContactLocationPayload,
	UpdateContactPayload,
} from '$lib/tauri/types';

// Contacts store — Phase E.
//
// Holds the flat list of contacts plus the last-loaded search query so
// re-renders don't forget the filter. Mutations reload the list; this is
// fine for the expected scale (tens to low hundreds of contacts per
// family business).

function createContactStore() {
	const { subscribe, set } = writable<Contact[]>([]);
	let lastSearch: string | undefined = undefined;

	async function load(search?: string): Promise<void> {
		lastSearch = search;
		try {
			set(await commands.getContacts(search));
		} catch (e) {
			console.error('Failed to load contacts:', e);
			set([]);
		}
	}

	return {
		subscribe,
		load,
		async create(payload: CreateContactPayload): Promise<string> {
			const id = await commands.createContact(payload);
			await load(lastSearch);
			return id;
		},
		async update(payload: UpdateContactPayload): Promise<void> {
			await commands.updateContact(payload);
			await load(lastSearch);
		},
		async remove(contactId: string): Promise<void> {
			await commands.deleteContact(contactId);
			await load(lastSearch);
		},
		async get(contactId: string): Promise<Contact | null> {
			return commands.getContact(contactId);
		},
		async getOrdersFor(contactId: string): Promise<Order[]> {
			return commands.getOrdersForContact(contactId);
		},

		// Locations — these don't reload the contacts list because the list
		// only shows the *default* address. Callers that need the fresh list
		// of locations for a specific contact should call listLocations().
		async listLocations(contactId: string): Promise<ContactLocation[]> {
			return commands.getContactLocations(contactId);
		},
		async addLocation(payload: CreateContactLocationPayload): Promise<string> {
			return commands.addContactLocation(payload);
		},
		async updateLocation(payload: UpdateContactLocationPayload): Promise<void> {
			return commands.updateContactLocation(payload);
		},
		async removeLocation(locationId: string): Promise<void> {
			return commands.deleteContactLocation(locationId);
		},
		async setDefaultLocation(locationId: string): Promise<void> {
			await commands.setDefaultContactLocation(locationId);
			// Default may bubble into the list's `default_address` column,
			// so refresh the list too.
			await load(lastSearch);
		},

		async uploadPhoto(contactId: string, sourcePath: string): Promise<string> {
			const path = await commands.saveContactPhoto(contactId, sourcePath);
			await load(lastSearch);
			return path;
		},

		async backfillFromOrders(): Promise<{ created: number; linked: number }> {
			const r = await commands.backfillContactsFromOrders();
			await load(lastSearch);
			return r;
		},
	};
}

export const contacts = createContactStore();
