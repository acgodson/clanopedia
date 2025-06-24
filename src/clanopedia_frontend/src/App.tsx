import { BrowserRouter } from 'react-router-dom';
import { AuthProvider } from './providers/useAuth';
import { ToastProvider } from './providers/toast';
import { ThemeProvider } from './providers/theme';
import { AppRoutes } from './routes';

function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <AuthProvider>
          <ToastProvider>
            <AppRoutes />
          </ToastProvider>
        </AuthProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}

export default App;
