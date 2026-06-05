import type {
  IntelligenceMessage,
  SmartMoneyAlert,
  SwapEvent,
  WalletSnapshot,
  WhaleAlert,
} from './types';
import { isIntelligenceBatch } from './types';

const MAX_SWAPS = 120;
const MAX_WHALES = 50;
const MAX_WALLETS = 40;

type Listener = () => void;

class IntelligenceStore {
  private version = 0;
  private listeners = new Set<Listener>();
  private swaps: SwapEvent[] = [];
  private whales: WhaleAlert[] = [];
  private smart: SmartMoneyAlert[] = [];
  private wallets = new Map<string, WalletSnapshot>();
  private connected = false;

  private swapCache: SwapEvent[] | null = null;
  private whaleCache: WhaleAlert[] | null = null;
  private walletCache: WalletSnapshot[] | null = null;
  private cacheVersion = -1;

  setConnected(v: boolean) {
    if (this.connected === v) return;
    this.connected = v;
    this.bump();
  }

  ingestRaw(text: string) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch {
      return;
    }
    if (!isIntelligenceBatch(parsed)) return;

    for (const msg of parsed.messages) {
      this.apply(msg);
    }
    this.bump();
  }

  private apply(msg: IntelligenceMessage) {
    switch (msg.type) {
      case 'swap':
      case 'enriched_swap':
        this.swaps = [...this.swaps, msg.payload].slice(-MAX_SWAPS);
        break;
      case 'whale_alert':
        this.whales = [msg.payload, ...this.whales].slice(0, MAX_WHALES);
        break;
      case 'smart_money_alert':
        this.smart = [msg.payload, ...this.smart].slice(0, MAX_WHALES);
        break;
      case 'wallet_snapshot':
        this.wallets.set(msg.payload.wallet, msg.payload);
        break;
      default:
        break;
    }
  }

  private bump() {
    this.version += 1;
    this.swapCache = null;
    this.whaleCache = null;
    this.walletCache = null;
    this.listeners.forEach((l) => l());
  }

  subscribe(l: Listener) {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  }

  get connectedSnapshot() {
    void this.version;
    return this.connected;
  }

  get swapsSnapshot(): SwapEvent[] {
    if (this.cacheVersion !== this.version || !this.swapCache) {
      this.swapCache = this.swaps;
      this.cacheVersion = this.version;
    }
    return this.swapCache;
  }

  get whalesSnapshot(): WhaleAlert[] {
    if (this.cacheVersion !== this.version || !this.whaleCache) {
      this.whaleCache = this.whales;
    }
    return this.whaleCache ?? [];
  }

  get smartSnapshot(): SmartMoneyAlert[] {
    void this.version;
    return this.smart;
  }

  get walletsSnapshot(): WalletSnapshot[] {
    if (this.cacheVersion !== this.version || !this.walletCache) {
      this.walletCache = Array.from(this.wallets.values())
        .sort((a, b) => b.volume_sol_24h - a.volume_sol_24h)
        .slice(0, MAX_WALLETS);
    }
    return this.walletCache ?? [];
  }
}

export const intelligenceStore = new IntelligenceStore();
