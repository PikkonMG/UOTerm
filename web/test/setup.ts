import { cleanup } from '@testing-library/preact';
import { afterEach } from 'vitest';

// Each test starts with an empty page: what one rendered goes after it.
afterEach(cleanup);
