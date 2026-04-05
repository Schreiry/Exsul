/**
 * Damped harmonic oscillator for spring-physics animations.
 *
 * x''(t) = -stiffness * (x - target) - damping * x'(t)
 *
 * Uses semi-implicit Euler integration at 60fps timestep.
 */

export interface SpringConfig {
	stiffness: number;
	damping: number;
	mass: number;
	precision: number;
}

export const DOCK_SPRING: SpringConfig = {
	stiffness: 220,
	damping: 22,
	mass: 1,
	precision: 0.01,
};

export const QUICK_SPRING: SpringConfig = {
	stiffness: 300,
	damping: 28,
	mass: 1,
	precision: 0.01,
};

export const GENTLE_SPRING: SpringConfig = {
	stiffness: 120,
	damping: 18,
	mass: 1,
	precision: 0.01,
};

export interface SpringState {
	value: number;
	velocity: number;
	target: number;
	done: boolean;
}

export function stepSpring(
	state: SpringState,
	config: SpringConfig,
	dt: number = 1 / 60
): SpringState {
	const { stiffness, damping, mass, precision } = config;
	const displacement = state.value - state.target;
	const springForce = -stiffness * displacement;
	const dampingForce = -damping * state.velocity;
	const acceleration = (springForce + dampingForce) / mass;

	const velocity = state.velocity + acceleration * dt;
	const value = state.value + velocity * dt;

	const done =
		Math.abs(velocity) < precision && Math.abs(value - state.target) < precision;

	return {
		value: done ? state.target : value,
		velocity: done ? 0 : velocity,
		target: state.target,
		done,
	};
}

export function createSpring(initial: number, config: SpringConfig = DOCK_SPRING) {
	let state: SpringState = {
		value: initial,
		velocity: 0,
		target: initial,
		done: true,
	};

	const subscribers = new Set<(v: number) => void>();
	let frameId: number | null = null;

	function notify() {
		for (const fn of subscribers) fn(state.value);
	}

	function tick() {
		state = stepSpring(state, config);
		notify();
		if (!state.done) {
			frameId = requestAnimationFrame(tick);
		} else {
			frameId = null;
		}
	}

	return {
		subscribe(fn: (v: number) => void) {
			subscribers.add(fn);
			fn(state.value);
			return () => {
				subscribers.delete(fn);
			};
		},
		set(target: number) {
			state.target = target;
			if (state.done && state.value !== target) {
				state.done = false;
				if (!frameId) frameId = requestAnimationFrame(tick);
			}
		},
		snap(value: number) {
			state = { value, velocity: 0, target: value, done: true };
			if (frameId) {
				cancelAnimationFrame(frameId);
				frameId = null;
			}
			notify();
		},
		get value() {
			return state.value;
		},
	};
}
