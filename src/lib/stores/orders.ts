import { writable } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type {
	AddOrderItemPayload,
	CreateOrderPayload,
	Order,
	OrderItem,
} from '$lib/tauri/types';

function createOrderStore() {
	const { subscribe, set } = writable<Order[]>([]);

	async function load(statusFilter?: string) {
		try {
			set(await commands.getOrders(statusFilter));
		} catch (e) {
			console.error('Failed to load orders:', e);
		}
	}

	return {
		subscribe,
		load,
		async create(payload: CreateOrderPayload): Promise<string> {
			const id = await commands.createOrder(payload);
			await load();
			return id;
		},
		async updateStatus(orderId: string, status: string): Promise<void> {
			await commands.updateOrderStatus(orderId, status);
			await load();
		},
		async addItem(payload: AddOrderItemPayload): Promise<string> {
			const id = await commands.addOrderItem(payload);
			await load();
			return id;
		},
		async getItems(orderId: string): Promise<OrderItem[]> {
			return commands.getOrderItems(orderId);
		},
	};
}

export const orders = createOrderStore();
