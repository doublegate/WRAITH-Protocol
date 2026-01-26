import React from 'react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
    variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
    size?: 'sm' | 'md' | 'lg';
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
    ({ className = '', variant = 'primary', size = 'md', children, ...props }, ref) => {
        const baseStyles = "font-bold rounded transition-all focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-slate-900";
        
        const variants = {
            primary: "bg-red-600 text-white hover:bg-red-500 shadow-lg shadow-red-900/20 focus:ring-red-500",
            secondary: "bg-slate-800 text-white hover:bg-slate-700 border border-slate-700 focus:ring-slate-500",
            danger: "bg-red-950 text-red-200 hover:bg-red-900 border border-red-900/50 focus:ring-red-500",
            ghost: "bg-transparent text-slate-400 hover:text-white hover:bg-slate-800",
        };

        const sizes = {
            sm: "px-2 py-1 text-xs",
            md: "px-4 py-2 text-sm",
            lg: "px-6 py-3 text-base",
        };

        return (
            <button
                ref={ref}
                className={`${baseStyles} ${variants[variant]} ${sizes[size]} ${className}`}
                {...props}
            >
                {children}
            </button>
        );
    }
);

Button.displayName = "Button";
