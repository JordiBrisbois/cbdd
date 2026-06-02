import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Toaster } from 'react-hot-toast'
import './index.css'
import App from './App'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
    <Toaster position="bottom-right" toastOptions={{ duration: 3000, style: { borderRadius: '0.75rem', fontSize: '0.875rem' } }} />
  </StrictMode>,
)