// SDK Entry Point
export * from './types';
export * from './server';
export * from './mediaEngine';

// Log when starting in development mode
if (process.env.NODE_ENV !== 'production') {
  console.log('Veil SDK development mode initialized');
} 