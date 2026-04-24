import { writable } from 'svelte/store';
import { commands } from '$lib/tauri/commands';
import type {
	AddOrderItemPayload,
	CreateOrderPayload,
	Order,
	OrderItem,
	UpdateOrderPayload,
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
		async update(payload: UpdateOrderPayload): Promise<void> {
			await commands.updateOrder(payload);
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
		async getEarliestDate(): Promise<string | null> {
			return commands.getEarliestOrderDate();
		},
		async remove(orderId: string): Promise<void> {
			await commands.deleteOrder(orderId);
			await load();
		},
		async removeAll(): Promise<number> {
			const count = await commands.deleteAllOrders();
			await load();
			return count;
		},
	};
}

export const orders = createOrderStore();
