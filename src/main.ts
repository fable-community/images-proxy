import { serve } from 'https://deno.land/std@0.187.0/http/server.ts';

import { handler } from '../build/images_proxy.js';

serve(handler);
