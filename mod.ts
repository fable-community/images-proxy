import {proxy as _proxy} from './build/images_proxy.js';

export function proxy(url: string, size?: 'large' | 'medium' | 'thumbnail' | 'preview'): Promise<{
  format: string,
  image: Uint8Array
}> {
  return _proxy(url, size) as any;
}